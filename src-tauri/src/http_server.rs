use crate::config::AppConfig;
use crate::error::{AppError, Result};
use crate::limiter::RateLimiter;
use crate::models::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChunkChoice, Delta,
    UsageLog,
};
use crate::provider::registry::ProviderRegistry;
use crate::repository::{
    DecisionChainRepository, ProviderRepository, SessionRepository, UsageLogRepository,
};
use crate::router::circuit_breaker::CircuitBreaker;
use crate::router::load_balancer::LoadBalancer;
use crate::router::request_router::RequestRouter;
use crate::session::SessionManager;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Response, Sse,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::convert::Infallible;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

/// Application state shared across all HTTP handlers
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub request_router: Arc<RequestRouter>,
    pub session_manager: Arc<SessionManager>,
    pub rate_limiter: Arc<RateLimiter>,
    pub provider_repository: Arc<ProviderRepository>,
    pub usage_log_repository: Arc<UsageLogRepository>,
}

/// Start the HTTP server on localhost:8080
/// Requirements: 4.1
pub async fn start_server(app_handle: AppHandle) -> Result<()> {
    tracing::info!("Initializing HTTP server...");

    // Load configuration
    let config = AppConfig::load()
        .map_err(|e| AppError::Config(format!("Failed to load configuration: {}", e)))?;

    tracing::info!("Configuration loaded successfully");
    tracing::debug!(
        "Server config: {}:{}",
        config.server.host,
        config.server.port
    );

    // Determine database path
    let db_path =
        if config.database.path.starts_with("~/") || config.database.path.starts_with("~\\") {
            // Expand tilde to home directory
            let expanded = AppConfig::expand_path(&config.database.path);
            std::path::PathBuf::from(expanded)
        } else if std::path::Path::new(&config.database.path).is_absolute() {
            // Use absolute path as-is
            std::path::PathBuf::from(&config.database.path)
        } else {
            // Use app data directory for relative paths
            let app_dir = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| AppError::Config(format!("Failed to get app data dir: {}", e)))?;
            app_dir.join(&config.database.path)
        };

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::Config(format!("Failed to create database directory: {}", e)))?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    // Initialize database pool
    let db = SqlitePool::connect(&db_url).await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(&db)
        .await
        .map_err(|e| AppError::Internal(format!("Migration failed: {}", e)))?;

    tracing::info!("Database initialized at: {}", db_path.display());

    // Initialize repositories
    let provider_repository = Arc::new(ProviderRepository::new(db.clone()));
    let session_repository = Arc::new(SessionRepository::new(db.clone()));
    let usage_log_repository = Arc::new(UsageLogRepository::new(db.clone()));
    let decision_chain_repository = Arc::new(DecisionChainRepository::new(db.clone()));

    // Initialize core components with configuration
    let rate_limiter = Arc::new(RateLimiter::new(true)); // Enable fail-open mode
    let circuit_breaker = Arc::new(CircuitBreaker::new(config.circuit_breaker.clone()));
    let load_balancer = Arc::new(LoadBalancer::new(Default::default()));
    let provider_registry = Arc::new(ProviderRegistry::new());
    let session_ttl_secs = (config.session.ttl_minutes * 60) as i64;
    let session_manager = Arc::new(SessionManager::new(
        session_repository.clone(),
        decision_chain_repository.clone(),
        session_ttl_secs,
    ));

    // Initialize request router
    let request_router = Arc::new(RequestRouter::new(
        load_balancer,
        circuit_breaker,
        provider_registry,
        provider_repository.clone(),
        session_manager.clone(),
        rate_limiter.clone(),
        db.clone(),
    ));

    // Start session cleanup task
    session_manager.clone().start_cleanup_task();

    // Create application state
    let state = AppState {
        db,
        request_router,
        session_manager,
        rate_limiter,
        provider_repository,
        usage_log_repository,
    };

    // Build router with all endpoints
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .route("/v1/models", get(models_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(axum::middleware::from_fn(error_handler_middleware))
        .with_state(state);

    // Start server with configured host and port
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("HTTP server listening on http://{}", addr);

    axum::serve(listener, app)
        .await
        .map_err(|e| AppError::Internal(format!("Server error: {}", e)))?;

    Ok(())
}

/// POST /v1/chat/completions - OpenAI compatible chat completions endpoint
/// Requirements: 4.1, 4.3
async fn chat_completions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> std::result::Result<Response, AppError> {
    tracing::info!(
        "Received chat completion request for model: {}, stream: {}",
        request.model,
        request.stream
    );

    // Extract session ID from headers if present
    let session_id = headers
        .get("x-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Handle streaming vs non-streaming
    if request.stream {
        // Return streaming response
        let stream = create_completion_stream(state, request, session_id).await?;
        Ok(Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response())
    } else {
        // Route request through router
        let (response, final_session_id, _decision_chain) = state
            .request_router
            .route_request(&request, session_id)
            .await?;

        // Log usage
        let usage_log = UsageLog::new(
            Some(final_session_id.clone()),
            response.model.clone(),
            response.model.clone(),
            &response.usage,
        );

        if let Err(e) = state.usage_log_repository.create(&usage_log).await {
            tracing::warn!("Failed to log usage: {}", e);
        }

        // Persist decision chain
        if let Err(e) = state
            .session_manager
            .persist_decision_chain(&final_session_id, &usage_log.id)
            .await
        {
            tracing::warn!("Failed to persist decision chain: {}", e);
        }

        tracing::info!(
            "Request completed successfully, session: {}, tokens: {}",
            final_session_id,
            response.usage.total_tokens
        );

        Ok(Json(response).into_response())
    }
}

/// Create a streaming response for chat completions
/// Requirements: 4.2
async fn create_completion_stream(
    state: AppState,
    request: ChatCompletionRequest,
    session_id: Option<String>,
) -> std::result::Result<impl Stream<Item = std::result::Result<Event, Infallible>>, AppError> {
    // For now, route the request normally and convert to stream
    // In a full implementation, this would stream from the provider
    let (response, final_session_id, _decision_chain) = state
        .request_router
        .route_request(&request, session_id)
        .await?;

    // Log usage
    let usage_log = UsageLog::new(
        Some(final_session_id.clone()),
        response.model.clone(),
        response.model.clone(),
        &response.usage,
    );

    if let Err(e) = state.usage_log_repository.create(&usage_log).await {
        tracing::warn!("Failed to log usage: {}", e);
    }

    // Persist decision chain
    if let Err(e) = state
        .session_manager
        .persist_decision_chain(&final_session_id, &usage_log.id)
        .await
    {
        tracing::warn!("Failed to persist decision chain: {}", e);
    }

    // Convert response to streaming chunks
    let chunks = convert_response_to_chunks(response);

    let stream = stream::iter(chunks).map(|chunk| {
        let data = serde_json::to_string(&chunk).unwrap_or_default();
        Ok::<_, Infallible>(Event::default().data(data))
    });

    Ok(stream)
}

/// Convert a complete response to streaming chunks
fn convert_response_to_chunks(response: ChatCompletionResponse) -> Vec<ChatCompletionChunk> {
    let mut chunks = Vec::new();

    // First chunk with role
    if let Some(choice) = response.choices.first() {
        chunks.push(ChatCompletionChunk::new(
            response.id.clone(),
            response.model.clone(),
            vec![ChunkChoice {
                index: 0,
                delta: Delta {
                    role: Some(choice.message.role.clone()),
                    content: None,
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
        ));

        // Content chunks (split into smaller pieces for streaming effect)
        if let Some(content) = &choice.message.content {
            let words: Vec<&str> = content.split_whitespace().collect();
            for word in words {
                chunks.push(ChatCompletionChunk::new(
                    response.id.clone(),
                    response.model.clone(),
                    vec![ChunkChoice {
                        index: 0,
                        delta: Delta {
                            role: None,
                            content: Some(format!("{} ", word)),
                            tool_calls: None,
                        },
                        finish_reason: None,
                        logprobs: None,
                    }],
                ));
            }
        }

        // Final chunk with finish_reason
        chunks.push(ChatCompletionChunk::new(
            response.id.clone(),
            response.model.clone(),
            vec![ChunkChoice {
                index: 0,
                delta: Delta {
                    role: None,
                    content: None,
                    tool_calls: None,
                },
                finish_reason: choice.finish_reason.clone(),
                logprobs: None,
            }],
        ));
    }

    // Done chunk
    chunks.push(ChatCompletionChunk::new(
        response.id,
        response.model,
        vec![],
    ));

    chunks
}

/// GET /v1/models - List available models
/// Requirements: 4.1
async fn models_handler(
    State(state): State<AppState>,
) -> std::result::Result<Json<ModelsResponse>, AppError> {
    let providers = state.provider_repository.get_enabled().await?;

    let models: Vec<ModelInfo> = providers
        .into_iter()
        .map(|p| ModelInfo {
            id: format!("{}/{}", p.provider_type, p.name),
            object: "model".to_string(),
            created: p.created_at,
            owned_by: p.provider_type.to_string(),
        })
        .collect();

    Ok(Json(ModelsResponse {
        object: "list".to_string(),
        data: models,
    }))
}

/// GET /health - Health check endpoint
/// Requirements: 5.4
async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_healthy = state.db.acquire().await.is_ok();
    let active_sessions = state.session_manager.active_session_count();

    Json(HealthResponse {
        status: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
        database: db_healthy,
        active_sessions,
        timestamp: chrono::Utc::now().timestamp(),
    })
}

/// GET /metrics - Prometheus-style metrics endpoint
/// Requirements: 5.4
async fn metrics_handler(State(state): State<AppState>) -> std::result::Result<String, AppError> {
    let active_sessions = state.session_manager.active_session_count();

    // Get recent usage stats (last hour)
    let one_hour_ago = chrono::Utc::now().timestamp() - 3600;
    let now = chrono::Utc::now().timestamp();

    // Query aggregated stats from the last hour
    let stats_row = sqlx::query(
        r#"
        SELECT 
            COUNT(*) as total_requests,
            SUM(CASE WHEN status != 'success' THEN 1 ELSE 0 END) as failed_requests,
            COALESCE(SUM(total_tokens), 0) as total_tokens,
            COALESCE(AVG(latency_ms), 0.0) as avg_latency
        FROM usage_logs
        WHERE created_at >= ? AND created_at <= ?
        "#,
    )
    .bind(one_hour_ago)
    .bind(now)
    .fetch_one(&state.db)
    .await?;

    use sqlx::Row;
    let total_requests: i64 = stats_row.try_get("total_requests")?;
    let failed_requests: i64 = stats_row.try_get("failed_requests")?;
    let total_tokens: i64 = stats_row.try_get("total_tokens")?;
    let avg_latency: f64 = stats_row.try_get("avg_latency")?;

    // Format as Prometheus metrics
    let metrics = format!(
        "# HELP llm_gateway_active_sessions Number of active sessions\n\
         # TYPE llm_gateway_active_sessions gauge\n\
         llm_gateway_active_sessions {}\n\
         \n\
         # HELP llm_gateway_requests_total Total number of requests in the last hour\n\
         # TYPE llm_gateway_requests_total counter\n\
         llm_gateway_requests_total {}\n\
         \n\
         # HELP llm_gateway_failed_requests_total Total number of failed requests in the last hour\n\
         # TYPE llm_gateway_failed_requests_total counter\n\
         llm_gateway_failed_requests_total {}\n\
         \n\
         # HELP llm_gateway_tokens_total Total tokens consumed in the last hour\n\
         # TYPE llm_gateway_tokens_total counter\n\
         llm_gateway_tokens_total {}\n\
         \n\
         # HELP llm_gateway_avg_latency_ms Average request latency in milliseconds\n\
         # TYPE llm_gateway_avg_latency_ms gauge\n\
         llm_gateway_avg_latency_ms {:.2}\n",
        active_sessions, total_requests, failed_requests, total_tokens, avg_latency
    );

    Ok(metrics)
}

/// Error handling middleware
/// Requirements: 11.1, 11.4, 11.5
async fn error_handler_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let response = next.run(req).await;
    response
}

// ============================================================================
// Response Models
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ModelsResponse {
    object: String,
    data: Vec<ModelInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelInfo {
    id: String,
    object: String,
    created: i64,
    owned_by: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct HealthResponse {
    status: String,
    database: bool,
    active_sessions: usize,
    timestamp: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorDetail {
    code: String,
    message: String,
    r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after: Option<u64>,
}

// ============================================================================
// Error Response Implementation
// ============================================================================

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_response) = match self {
            AppError::RateLimit(ref e) => {
                let (code, retry_after) = match e {
                    crate::error::RateLimitError::RpmExceeded {
                        retry_after_secs, ..
                    } => ("rate_limit_exceeded", Some(*retry_after_secs)),
                    crate::error::RateLimitError::TokenQuotaExceeded { .. } => {
                        ("token_quota_exceeded", None)
                    }
                    crate::error::RateLimitError::ConcurrencyExceeded(_) => {
                        ("concurrency_exceeded", None)
                    }
                    _ => ("rate_limit_error", None),
                };

                (
                    StatusCode::TOO_MANY_REQUESTS,
                    ErrorResponse {
                        error: ErrorDetail {
                            code: code.to_string(),
                            message: self.to_string(),
                            r#type: "rate_limit_error".to_string(),
                            retry_after,
                        },
                    },
                )
            }
            AppError::Router(ref e) => {
                let code = match e {
                    crate::error::RouterError::NoAvailableBackends(_) => "no_available_backends",
                    crate::error::RouterError::AllBackendsFailed(_) => "all_backends_failed",
                    crate::error::RouterError::ModelNotFound(_) => "model_not_found",
                    _ => "router_error",
                };

                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    ErrorResponse {
                        error: ErrorDetail {
                            code: code.to_string(),
                            message: self.to_string(),
                            r#type: "router_error".to_string(),
                            retry_after: None,
                        },
                    },
                )
            }
            AppError::CircuitBreakerOpen(_) => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorResponse {
                    error: ErrorDetail {
                        code: "circuit_breaker_open".to_string(),
                        message: self.to_string(),
                        r#type: "circuit_breaker_error".to_string(),
                        retry_after: Some(30),
                    },
                },
            ),
            AppError::Provider(ref e) => {
                let code = match e {
                    crate::error::ProviderError::NotFound(_) => "provider_not_found",
                    crate::error::ProviderError::Disabled(_) => "provider_disabled",
                    crate::error::ProviderError::AuthenticationFailed(_) => "authentication_failed",
                    crate::error::ProviderError::Timeout(_) => "provider_timeout",
                    _ => "provider_error",
                };

                (
                    StatusCode::BAD_GATEWAY,
                    ErrorResponse {
                        error: ErrorDetail {
                            code: code.to_string(),
                            message: self.to_string(),
                            r#type: "provider_error".to_string(),
                            retry_after: None,
                        },
                    },
                )
            }
            AppError::Validation(_) | AppError::InvalidInput(_) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: ErrorDetail {
                        code: "invalid_request".to_string(),
                        message: self.to_string(),
                        r#type: "validation_error".to_string(),
                        retry_after: None,
                    },
                },
            ),
            AppError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: ErrorDetail {
                        code: "not_found".to_string(),
                        message: self.to_string(),
                        r#type: "not_found_error".to_string(),
                        retry_after: None,
                    },
                },
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse {
                    error: ErrorDetail {
                        code: "internal_error".to_string(),
                        message: "An internal error occurred".to_string(),
                        r#type: "internal_error".to_string(),
                        retry_after: None,
                    },
                },
            ),
        };

        tracing::error!("HTTP error response: {} - {}", status, self);

        (status, Json(error_response)).into_response()
    }
}
