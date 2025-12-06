use crate::config::AppConfig;
use crate::crypto::CryptoService;
use crate::models::{
    DashboardMetrics, DecisionRecord, ModelPricing, ModelRedirect, Provider, RateLimitConfig,
    SessionInfo,
};
use crate::provider::registry::ProviderRegistry;
use crate::repository::{
    DecisionChainRepository, ModelPricingRepository, ModelRedirectRepository, ProviderRepository,
};
use crate::validation;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{fs, path::PathBuf, sync::Arc};
use tauri::{AppHandle, Manager};

// ============================================================================
// Application State for Tauri Commands
// ============================================================================

pub struct CommandState {
    pub db: SqlitePool,
    pub crypto: Arc<CryptoService>,
    pub provider_registry: Arc<ProviderRegistry>,
}

// ============================================================================
// Task 13.1: Provider Management Commands
// ============================================================================

/// Get all providers
/// Requirements: 1.1
#[tauri::command]
pub async fn get_providers(app: AppHandle) -> Result<Vec<Provider>, String> {
    let db = get_db_pool(&app).await?;
    let crypto = get_crypto_service()?;
    let repo = ProviderRepository::new(db);

    let mut providers = repo.get_all().await.map_err(|e| e.to_string())?;

    // Decrypt API keys for display (in production, you might want to mask these)
    for provider in &mut providers {
        if let Ok(decrypted) = crypto.decrypt_from_base64(&provider.api_key) {
            provider.api_key = decrypted;
        }
    }

    Ok(providers)
}

/// Create a new provider
/// Requirements: 1.1, 1.2
#[tauri::command]
pub async fn create_provider(provider: Provider, app: AppHandle) -> Result<Provider, String> {
    // Validate provider configuration
    validation::validate_provider(&provider)?;

    let db = get_db_pool(&app).await?;
    let crypto = get_crypto_service()?;
    let repo = ProviderRepository::new(db);

    // Encrypt API key before storing
    let mut provider_to_store = provider.clone();
    provider_to_store.api_key = crypto
        .encrypt_to_base64(&provider.api_key)
        .map_err(|e| format!("Failed to encrypt API key: {}", e))?;

    // Store in database
    repo.create(&provider_to_store)
        .await
        .map_err(|e| e.to_string())?;

    // Return provider with original (unencrypted) API key
    Ok(provider)
}

/// Update an existing provider
/// Requirements: 1.1, 1.2
#[tauri::command]
pub async fn update_provider(provider: Provider, app: AppHandle) -> Result<Provider, String> {
    // Validate provider configuration
    validation::validate_provider(&provider)?;

    let db = get_db_pool(&app).await?;
    let crypto = get_crypto_service()?;
    let repo = ProviderRepository::new(db);

    // Encrypt API key before storing
    let mut provider_to_store = provider.clone();
    provider_to_store.api_key = crypto
        .encrypt_to_base64(&provider.api_key)
        .map_err(|e| format!("Failed to encrypt API key: {}", e))?;

    // Update in database
    repo.update(&provider_to_store)
        .await
        .map_err(|e| e.to_string())?;

    // Return provider with original (unencrypted) API key
    Ok(provider)
}

/// Delete a provider
/// Requirements: 1.1
#[tauri::command]
pub async fn delete_provider(provider_id: String, app: AppHandle) -> Result<(), String> {
    let db = get_db_pool(&app).await?;
    let repo = ProviderRepository::new(db);

    repo.delete(&provider_id).await.map_err(|e| e.to_string())?;

    Ok(())
}

/// Test provider connection by sending a simple request
/// Requirements: 1.2, 1.4
#[tauri::command]
pub async fn test_provider_connection(
    provider_id: Option<String>,
    provider: Option<Provider>,
    app: AppHandle,
) -> Result<bool, String> {
    let registry = ProviderRegistry::new();

    // Determine which provider configuration to test
    let provider = if let Some(provider_config) = provider {
        provider_config
    } else {
        let provider_id = provider_id
            .ok_or_else(|| "Provider ID or configuration is required for testing".to_string())?;

        let db = get_db_pool(&app).await?;
        let crypto = get_crypto_service()?;
        let repo = ProviderRepository::new(db);

        let mut stored_provider = repo
            .get_by_id(&provider_id)
            .await
            .map_err(|e| e.to_string())?;

        stored_provider.api_key = crypto
            .decrypt_from_base64(&stored_provider.api_key)
            .map_err(|e| format!("Failed to decrypt API key: {}", e))?;

        stored_provider
    };

    // Ensure configuration is valid before testing
    validation::validate_provider(&provider)?;

    // Get adapter for provider type
    let adapter = registry
        .get_adapter(&provider.provider_type)
        .map_err(|e| format!("No adapter found for provider type: {}", e))?;

    // Validate configuration (this will test the connection)
    adapter
        .validate_config(&provider)
        .await
        .map_err(|e| format!("Connection test failed: {}", e))?;

    Ok(true)
}

// ============================================================================
// Task 13.2: Monitoring Commands
// ============================================================================

/// Get dashboard metrics
/// Requirements: 5.1, 5.2
#[tauri::command]
pub async fn get_dashboard_metrics(app: AppHandle) -> Result<DashboardMetrics, String> {
    let db = get_db_pool(&app).await?;

    // Calculate time range (last hour)
    let one_hour_ago = chrono::Utc::now().timestamp() - 3600;
    let now = chrono::Utc::now().timestamp();

    // Query aggregated stats
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
    .fetch_one(&db)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let total_requests: i64 = stats_row
        .try_get("total_requests")
        .map_err(|e| e.to_string())?;
    let failed_requests: i64 = stats_row
        .try_get("failed_requests")
        .map_err(|e| e.to_string())?;
    let total_tokens: i64 = stats_row
        .try_get("total_tokens")
        .map_err(|e| e.to_string())?;
    let avg_latency: f64 = stats_row
        .try_get("avg_latency")
        .map_err(|e| e.to_string())?;

    // Calculate QPS (requests per second over the last hour)
    let qps = total_requests as f64 / 3600.0;

    // Calculate error rate
    let error_rate = if total_requests > 0 {
        failed_requests as f64 / total_requests as f64
    } else {
        0.0
    };

    // Count active sessions
    let active_sessions_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions WHERE expired = FALSE AND last_activity >= ?",
    )
    .bind(now - 300) // 5 minutes TTL
    .fetch_one(&db)
    .await
    .map_err(|e| e.to_string())?;

    Ok(DashboardMetrics {
        qps,
        error_rate,
        active_sessions: active_sessions_count as u32,
        total_requests_1h: total_requests as u64,
        total_tokens_1h: total_tokens as u64,
        avg_latency_ms: avg_latency,
    })
}

/// Get active sessions with provider information
/// Requirements: 5.2
#[tauri::command]
pub async fn get_active_sessions(app: AppHandle) -> Result<Vec<SessionInfo>, String> {
    let db = get_db_pool(&app).await?;

    let now = chrono::Utc::now().timestamp();
    let ttl = 300; // 5 minutes

    let rows = sqlx::query(
        r#"
        SELECT 
            s.id,
            s.backend_id,
            s.created_at,
            s.last_activity,
            p.name as provider_name
        FROM sessions s
        JOIN providers p ON s.backend_id = p.id
        WHERE s.expired = FALSE AND s.last_activity >= ?
        ORDER BY s.last_activity DESC
        "#,
    )
    .bind(now - ttl)
    .fetch_all(&db)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let sessions: Vec<SessionInfo> = rows
        .into_iter()
        .map(|row| {
            let created_at: i64 = row.get("created_at");
            let last_activity: i64 = row.get("last_activity");
            SessionInfo {
                id: row.get("id"),
                backend_id: row.get("backend_id"),
                provider_name: row.get("provider_name"),
                created_at,
                last_activity,
                duration_seconds: (now - created_at) as u64,
            }
        })
        .collect();

    Ok(sessions)
}

// ============================================================================
// Task 13.3: Configuration Management Commands
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct ExportConfig {
    providers: Vec<Provider>,
    rate_limits: Vec<RateLimitConfigExport>,
    version: String,
    exported_at: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct RateLimitConfigExport {
    provider_id: String,
    config: RateLimitConfig,
}

/// Export configuration to JSON
/// Requirements: 10.4
#[tauri::command]
pub async fn export_config(app: AppHandle) -> Result<String, String> {
    let db = get_db_pool(&app).await?;
    let crypto = get_crypto_service()?;
    let provider_repo = ProviderRepository::new(db.clone());

    // Get all providers
    let mut providers = provider_repo.get_all().await.map_err(|e| e.to_string())?;

    // Decrypt API keys
    for provider in &mut providers {
        if let Ok(decrypted) = crypto.decrypt_from_base64(&provider.api_key) {
            provider.api_key = decrypted;
        }
    }

    // Get rate limit configs
    let mut rate_limits = Vec::new();
    for provider in &providers {
        if let Ok(Some(config)) = get_rate_limit_config(&db, &provider.id).await {
            rate_limits.push(RateLimitConfigExport {
                provider_id: provider.id.clone(),
                config,
            });
        }
    }

    let export = ExportConfig {
        providers,
        rate_limits,
        version: "1.0".to_string(),
        exported_at: chrono::Utc::now().timestamp(),
    };

    serde_json::to_string_pretty(&export).map_err(|e| format!("Failed to serialize config: {}", e))
}

/// Import configuration from JSON
/// Requirements: 10.5
#[tauri::command]
pub async fn import_config(config_json: String, app: AppHandle) -> Result<(), String> {
    let db = get_db_pool(&app).await?;
    let crypto = get_crypto_service()?;
    let provider_repo = ProviderRepository::new(db.clone());

    // Parse JSON
    let import: ExportConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse config JSON: {}", e))?;

    // Import providers
    for provider in import.providers {
        // Validate
        validation::validate_provider(&provider)?;

        // Encrypt API key
        let mut provider_to_store = provider.clone();
        provider_to_store.api_key = crypto
            .encrypt_to_base64(&provider.api_key)
            .map_err(|e| format!("Failed to encrypt API key: {}", e))?;

        // Check if provider exists
        if provider_repo.get_by_id(&provider.id).await.is_ok() {
            // Update existing
            provider_repo
                .update(&provider_to_store)
                .await
                .map_err(|e| e.to_string())?;
        } else {
            // Create new
            provider_repo
                .create(&provider_to_store)
                .await
                .map_err(|e| e.to_string())?;
        }
    }

    // Import rate limit configs
    for rate_limit in import.rate_limits {
        save_rate_limit_config(&db, &rate_limit.provider_id, &rate_limit.config)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

// ============================================================================
// Task 13.4: Pricing Management Commands
// ============================================================================

/// Get all model pricing with pagination
/// Requirements: 7.1, 7.2
#[tauri::command]
pub async fn get_model_pricing(
    page: u32,
    page_size: u32,
    app: AppHandle,
) -> Result<Vec<ModelPricing>, String> {
    let db = get_db_pool(&app).await?;
    let repo = ModelPricingRepository::new(db);

    let offset = page * page_size;
    repo.get_paginated(page_size as i64, offset as i64)
        .await
        .map_err(|e| e.to_string())
}

/// Search model pricing by name
/// Requirements: 7.3
#[tauri::command]
pub async fn search_model_pricing(
    query: String,
    app: AppHandle,
) -> Result<Vec<ModelPricing>, String> {
    let db = get_db_pool(&app).await?;
    let repo = ModelPricingRepository::new(db);

    repo.search(&query).await.map_err(|e| e.to_string())
}

/// Sync pricing data from LiteLLM
/// Requirements: 7.4
#[tauri::command]
pub async fn sync_pricing_from_litellm(app: AppHandle) -> Result<u32, String> {
    let db = get_db_pool(&app).await?;
    let repo = ModelPricingRepository::new(db);

    // Fetch pricing data from LiteLLM API
    let client = reqwest::Client::new();
    let response = client
        .get("https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch pricing data: {}", e))?;

    let pricing_data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse pricing data: {}", e))?;

    // Parse and insert pricing data
    let mut count = 0;
    if let Some(models) = pricing_data.as_object() {
        for (model_name, data) in models {
            if let Some(obj) = data.as_object() {
                let input_cost = obj
                    .get("input_cost_per_token")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    * 1_000_000.0; // Convert to per 1M tokens

                let output_cost = obj
                    .get("output_cost_per_token")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    * 1_000_000.0;

                let provider = obj
                    .get("litellm_provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let pricing = ModelPricing {
                    id: uuid::Uuid::new_v4().to_string(),
                    model_name: model_name.clone(),
                    provider,
                    input_cost_per_1m: input_cost,
                    output_cost_per_1m: output_cost,
                    last_updated: chrono::Utc::now().timestamp(),
                };

                if repo.upsert(&pricing).await.is_ok() {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

// ============================================================================
// Dashboard: Circuit Breaker Status
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct CircuitBreakerStatus {
    pub provider_id: String,
    pub provider_name: String,
    pub state: String,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure: Option<i64>,
    pub opened_at: Option<i64>,
}

/// Get circuit breaker status for all providers
/// Requirements: 5.4
#[tauri::command]
pub async fn get_circuit_breaker_status(
    app: AppHandle,
) -> Result<Vec<CircuitBreakerStatus>, String> {
    let db = get_db_pool(&app).await?;

    let rows = sqlx::query(
        r#"
        SELECT 
            cb.provider_id,
            p.name as provider_name,
            cb.state,
            cb.failure_count,
            cb.success_count,
            cb.last_failure,
            cb.opened_at
        FROM circuit_breaker_states cb
        JOIN providers p ON cb.provider_id = p.id
        ORDER BY p.name
        "#,
    )
    .fetch_all(&db)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let statuses: Vec<CircuitBreakerStatus> = rows
        .into_iter()
        .map(|row| CircuitBreakerStatus {
            provider_id: row.get("provider_id"),
            provider_name: row.get("provider_name"),
            state: row.get("state"),
            failure_count: row.get::<i32, _>("failure_count") as u32,
            success_count: row.get::<i32, _>("success_count") as u32,
            last_failure: row.get("last_failure"),
            opened_at: row.get("opened_at"),
        })
        .collect();

    Ok(statuses)
}

// ============================================================================
// Dashboard: Consumption Rankings
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ConsumptionRanking {
    pub user_id: String,
    pub request_count: i64,
    pub total_tokens: i64,
    pub cost_usd: f64,
}

/// Get top consumers by request count and token usage
/// Requirements: 5.3
#[tauri::command]
pub async fn get_consumption_rankings(
    limit: u32,
    app: AppHandle,
) -> Result<Vec<ConsumptionRanking>, String> {
    let db = get_db_pool(&app).await?;

    // Query top consumers from usage logs
    // Note: In a real implementation, you'd track user_id in the request
    // For now, we'll group by session_id as a proxy for users
    let rows = sqlx::query(
        r#"
        SELECT 
            COALESCE(session_id, 'anonymous') as user_id,
            COUNT(*) as request_count,
            SUM(total_tokens) as total_tokens,
            SUM(cost_usd) as cost_usd
        FROM usage_logs
        WHERE created_at >= ?
        GROUP BY session_id
        ORDER BY total_tokens DESC
        LIMIT ?
        "#,
    )
    .bind(chrono::Utc::now().timestamp() - 86400) // Last 24 hours
    .bind(limit as i64)
    .fetch_all(&db)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let rankings: Vec<ConsumptionRanking> = rows
        .into_iter()
        .map(|row| ConsumptionRanking {
            user_id: row.get("user_id"),
            request_count: row.get("request_count"),
            total_tokens: row.get("total_tokens"),
            cost_usd: row.get("cost_usd"),
        })
        .collect();

    Ok(rankings)
}

// ============================================================================
// Task 13.5: Session Viewing Commands
// ============================================================================

/// Get session history with pagination
/// Requirements: 6.5
#[tauri::command]
pub async fn get_session_history(
    page: u32,
    page_size: u32,
    app: AppHandle,
) -> Result<Vec<SessionInfo>, String> {
    let db = get_db_pool(&app).await?;

    let offset = page * page_size;

    let rows = sqlx::query(
        r#"
        SELECT 
            s.id,
            s.backend_id,
            s.created_at,
            s.last_activity,
            p.name as provider_name
        FROM sessions s
        JOIN providers p ON s.backend_id = p.id
        ORDER BY s.created_at DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(page_size as i64)
    .bind(offset as i64)
    .fetch_all(&db)
    .await
    .map_err(|e| e.to_string())?;

    use sqlx::Row;
    let sessions: Vec<SessionInfo> = rows
        .into_iter()
        .map(|row| {
            let created_at: i64 = row.get("created_at");
            let last_activity: i64 = row.get("last_activity");
            SessionInfo {
                id: row.get("id"),
                backend_id: row.get("backend_id"),
                provider_name: row.get("provider_name"),
                created_at,
                last_activity,
                duration_seconds: (last_activity - created_at) as u64,
            }
        })
        .collect();

    Ok(sessions)
}

/// Get decision chain for a session
/// Requirements: 6.5
#[tauri::command]
pub async fn get_session_decision_chain(
    session_id: String,
    app: AppHandle,
) -> Result<Vec<DecisionRecord>, String> {
    let db = get_db_pool(&app).await?;
    let repo = DecisionChainRepository::new(db);

    // Get usage logs for this session
    let usage_logs: Vec<String> =
        sqlx::query_scalar("SELECT id FROM usage_logs WHERE session_id = ? ORDER BY created_at")
            .bind(&session_id)
            .fetch_all(&repo.db)
            .await
            .map_err(|e| e.to_string())?;

    // Get decision chains for all usage logs
    let mut all_decisions = Vec::new();
    for usage_log_id in usage_logs {
        if let Ok(decisions) = repo.get_by_usage_log(&usage_log_id).await {
            all_decisions.extend(decisions);
        }
    }

    Ok(all_decisions)
}

// ============================================================================
// Task 19: Settings Management Commands
// ============================================================================

/// Get global configuration
/// Requirements: 10.4
#[tauri::command]
pub async fn get_global_config(_app: AppHandle) -> Result<crate::config::AppConfig, String> {
    // Load configuration from file and environment variables
    crate::config::AppConfig::load().map_err(|e| format!("Failed to load configuration: {}", e))
}

/// Update global configuration
/// Requirements: 10.4
#[tauri::command]
pub async fn update_global_config(
    config: crate::config::AppConfig,
    _app: AppHandle,
) -> Result<(), String> {
    // Save configuration to file
    config
        .save()
        .map_err(|e| format!("Failed to save configuration: {}", e))
}

// ============================================================================
// Task 20.2: Model Redirect Management Commands
// ============================================================================

/// Get all model redirects
/// Requirements: 9.1
#[tauri::command]
pub async fn get_model_redirects(app: AppHandle) -> Result<Vec<ModelRedirect>, String> {
    let db = get_db_pool(&app).await?;
    let repo = ModelRedirectRepository::new(db);

    repo.get_all().await.map_err(|e| e.to_string())
}

/// Get a model redirect by ID
/// Requirements: 9.1
#[tauri::command]
pub async fn get_model_redirect(id: String, app: AppHandle) -> Result<ModelRedirect, String> {
    let db = get_db_pool(&app).await?;
    let repo = ModelRedirectRepository::new(db);

    repo.get_by_id(&id).await.map_err(|e| e.to_string())
}

/// Create a new model redirect
/// Requirements: 9.1, 9.5
#[tauri::command]
pub async fn create_model_redirect(
    redirect: ModelRedirect,
    app: AppHandle,
) -> Result<ModelRedirect, String> {
    let db = get_db_pool(&app).await?;
    let redirect_repo = ModelRedirectRepository::new(db.clone());
    let provider_repo = ProviderRepository::new(db.clone());

    // Validate that source model doesn't already have a redirect
    if redirect_repo
        .exists_for_source(&redirect.source_model, None)
        .await
        .map_err(|e| e.to_string())?
    {
        return Err(format!(
            "A redirect rule already exists for source model: {}",
            redirect.source_model
        ));
    }

    // Validate that target provider exists
    provider_repo
        .get_by_id(&redirect.target_provider_id)
        .await
        .map_err(|_| format!("Target provider not found: {}", redirect.target_provider_id))?;

    // Create the redirect
    redirect_repo
        .create(&redirect)
        .await
        .map_err(|e| e.to_string())?;

    Ok(redirect)
}

/// Update an existing model redirect
/// Requirements: 9.1, 9.4, 9.5
#[tauri::command]
pub async fn update_model_redirect(
    redirect: ModelRedirect,
    app: AppHandle,
) -> Result<ModelRedirect, String> {
    let db = get_db_pool(&app).await?;
    let redirect_repo = ModelRedirectRepository::new(db.clone());
    let provider_repo = ProviderRepository::new(db.clone());

    // Validate that source model doesn't already have a redirect (excluding current one)
    if redirect_repo
        .exists_for_source(&redirect.source_model, Some(&redirect.id))
        .await
        .map_err(|e| e.to_string())?
    {
        return Err(format!(
            "A redirect rule already exists for source model: {}",
            redirect.source_model
        ));
    }

    // Validate that target provider exists
    provider_repo
        .get_by_id(&redirect.target_provider_id)
        .await
        .map_err(|_| format!("Target provider not found: {}", redirect.target_provider_id))?;

    // Update the redirect
    redirect_repo
        .update(&redirect)
        .await
        .map_err(|e| e.to_string())?;

    Ok(redirect)
}

/// Delete a model redirect
/// Requirements: 9.1
#[tauri::command]
pub async fn delete_model_redirect(id: String, app: AppHandle) -> Result<(), String> {
    let db = get_db_pool(&app).await?;
    let repo = ModelRedirectRepository::new(db);

    repo.delete(&id).await.map_err(|e| e.to_string())
}

/// Toggle model redirect enabled status
/// Requirements: 9.4
#[tauri::command]
pub async fn toggle_model_redirect(
    id: String,
    enabled: bool,
    app: AppHandle,
) -> Result<(), String> {
    let db = get_db_pool(&app).await?;
    let repo = ModelRedirectRepository::new(db);

    repo.toggle_enabled(&id, enabled)
        .await
        .map_err(|e| e.to_string())
}

// ============================================================================
// Helper Functions
// ============================================================================

async fn get_db_pool(app: &AppHandle) -> Result<SqlitePool, String> {
    let config = AppConfig::load().map_err(|e| format!("Failed to load configuration: {}", e))?;
    let db_path = resolve_database_path(app, &config.database.path)?;

    // SQLite needs the directory to already exist before it can create the db file
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create database directory: {}", e))?;
    }

    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePool::connect(&db_url)
        .await
        .map_err(|e| format!("Failed to connect to database: {}", e))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| format!("Failed to run database migrations: {}", e))?;

    Ok(pool)
}

fn resolve_database_path(app: &AppHandle, configured_path: &str) -> Result<PathBuf, String> {
    if configured_path.starts_with("~/") || configured_path.starts_with("~\\") {
        return Ok(PathBuf::from(AppConfig::expand_path(configured_path)));
    }

    let provided_path = PathBuf::from(configured_path);
    if provided_path.is_absolute() {
        Ok(provided_path)
    } else {
        let app_dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Failed to get app data dir: {}", e))?;
        Ok(app_dir.join(provided_path))
    }
}

fn get_crypto_service() -> Result<Arc<CryptoService>, String> {
    CryptoService::new()
        .map(Arc::new)
        .map_err(|e| format!("Failed to initialize crypto service: {}", e))
}

async fn get_rate_limit_config(
    db: &SqlitePool,
    provider_id: &str,
) -> Result<Option<RateLimitConfig>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT rpm, tokens_per_5h, tokens_per_week, tokens_per_month, max_concurrent_sessions
        FROM rate_limit_configs
        WHERE provider_id = ?
        "#,
    )
    .bind(provider_id)
    .fetch_optional(db)
    .await?;

    if let Some(row) = row {
        use sqlx::Row;
        Ok(Some(RateLimitConfig {
            rpm: row.try_get("rpm").ok(),
            tokens_per_5h: row.try_get("tokens_per_5h").ok(),
            tokens_per_week: row.try_get("tokens_per_week").ok(),
            tokens_per_month: row.try_get("tokens_per_month").ok(),
            max_concurrent_sessions: row.try_get("max_concurrent_sessions").ok(),
        }))
    } else {
        Ok(None)
    }
}

async fn save_rate_limit_config(
    db: &SqlitePool,
    provider_id: &str,
    config: &RateLimitConfig,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO rate_limit_configs (provider_id, rpm, tokens_per_5h, tokens_per_week, tokens_per_month, max_concurrent_sessions)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(provider_id) DO UPDATE SET
            rpm = excluded.rpm,
            tokens_per_5h = excluded.tokens_per_5h,
            tokens_per_week = excluded.tokens_per_week,
            tokens_per_month = excluded.tokens_per_month,
            max_concurrent_sessions = excluded.max_concurrent_sessions
        "#,
    )
    .bind(provider_id)
    .bind(config.rpm)
    .bind(config.tokens_per_5h.map(|v| v as i64))
    .bind(config.tokens_per_week.map(|v| v as i64))
    .bind(config.tokens_per_month.map(|v| v as i64))
    .bind(config.max_concurrent_sessions)
    .execute(db)
    .await?;

    Ok(())
}

// ============================================================================
// Task 21: Log Viewing Commands
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    pub level: Option<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

/// Get recent log entries from the log file
/// Requirements: 11.4
#[tauri::command]
pub async fn get_logs(filter: LogFilter, _app: AppHandle) -> Result<Vec<LogEntry>, String> {
    let log_dir = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get data directory".to_string())?
        .join("velo-hub")
        .join("logs");

    // Find the most recent log file
    let log_files = std::fs::read_dir(&log_dir)
        .map_err(|e| format!("Failed to read log directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "log")
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    if log_files.is_empty() {
        return Ok(Vec::new());
    }

    // Get the most recent log file
    let mut log_files = log_files;
    log_files.sort_by_key(|entry| {
        entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    log_files.reverse();

    let log_file = &log_files[0];
    let content = std::fs::read_to_string(log_file.path())
        .map_err(|e| format!("Failed to read log file: {}", e))?;

    // Parse log entries
    let mut entries = Vec::new();
    for line in content.lines() {
        if let Some(entry) = parse_log_line(line) {
            // Apply filters
            if let Some(ref level_filter) = filter.level {
                if !entry.level.eq_ignore_ascii_case(level_filter) {
                    continue;
                }
            }

            if let Some(ref search) = filter.search {
                if !entry
                    .message
                    .to_lowercase()
                    .contains(&search.to_lowercase())
                {
                    continue;
                }
            }

            entries.push(entry);
        }
    }

    // Limit results
    let limit = filter.limit.unwrap_or(100);
    entries.truncate(limit);

    Ok(entries)
}

/// Get available log files
/// Requirements: 11.4
#[tauri::command]
pub async fn get_log_files(_app: AppHandle) -> Result<Vec<String>, String> {
    let log_dir = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get data directory".to_string())?
        .join("velo-hub")
        .join("logs");

    let log_files = std::fs::read_dir(&log_dir)
        .map_err(|e| format!("Failed to read log directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext == "log")
                .unwrap_or(false)
        })
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<_>>();

    Ok(log_files)
}

/// Get log directory path
/// Requirements: 11.4
#[tauri::command]
pub async fn get_log_directory(_app: AppHandle) -> Result<String, String> {
    let log_dir = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get data directory".to_string())?
        .join("velo-hub")
        .join("logs");

    Ok(log_dir.to_string_lossy().to_string())
}

/// Parse a log line into a LogEntry
fn parse_log_line(line: &str) -> Option<LogEntry> {
    // Simple parser for tracing-subscriber format
    // Format: 2024-01-01T12:00:00.000Z  INFO target: message file:line

    let parts: Vec<&str> = line.splitn(3, ' ').collect();
    if parts.len() < 3 {
        return None;
    }

    let timestamp = parts[0].trim().to_string();
    let level = parts[1].trim().to_string();
    let rest = parts[2];

    // Try to extract target and message
    let (target, message, file, line) = if let Some(colon_pos) = rest.find(':') {
        let target = rest[..colon_pos].trim().to_string();
        let after_colon = &rest[colon_pos + 1..];

        // Try to extract file and line from the end
        if let Some(file_pos) = after_colon.rfind(" at ") {
            let message = after_colon[..file_pos].trim().to_string();
            let file_info = &after_colon[file_pos + 4..];

            if let Some(line_pos) = file_info.rfind(':') {
                let file = Some(file_info[..line_pos].to_string());
                let line = file_info[line_pos + 1..].parse::<u32>().ok();
                (target, message, file, line)
            } else {
                (target, after_colon.trim().to_string(), None, None)
            }
        } else {
            (target, after_colon.trim().to_string(), None, None)
        }
    } else {
        ("unknown".to_string(), rest.to_string(), None, None)
    };

    Some(LogEntry {
        timestamp,
        level,
        target,
        message,
        file,
        line,
    })
}

/// Update log level dynamically
/// Requirements: 11.4
#[tauri::command]
pub async fn update_log_level(level: String, _app: AppHandle) -> Result<(), String> {
    // Validate log level
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&level.to_lowercase().as_str()) {
        return Err(format!(
            "Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
            level
        ));
    }

    tracing::info!("Log level updated to: {}", level);

    // Note: Dynamically updating the log level requires using tracing-subscriber's reload layer
    // For now, we just log the change. A full implementation would require restructuring
    // the logging initialization to support dynamic reloading.

    Ok(())
}
