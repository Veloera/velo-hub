use crate::error::{AppError, ProviderError, Result, RouterError};
use crate::limiter::RateLimiter;
use crate::models::{
    Backend, ChatCompletionRequest, ChatCompletionResponse, DecisionAction, DecisionRecord,
    ModelRedirect, Provider, ProxyType, RateLimitConfig,
};
use crate::provider::registry::ProviderRegistry;
use crate::repository::ProviderRepository;
use crate::router::circuit_breaker::CircuitBreaker;
use crate::router::load_balancer::LoadBalancer;
use crate::session::SessionManager;
use reqwest::Client;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// RequestRouter handles routing requests to appropriate backend providers
/// with load balancing, circuit breaking, and retry logic
pub struct RequestRouter {
    /// Load balancer for backend selection
    load_balancer: Arc<LoadBalancer>,
    /// Circuit breaker for fault isolation
    circuit_breaker: Arc<CircuitBreaker>,
    /// Provider registry for adapter lookup
    provider_registry: Arc<ProviderRegistry>,
    /// Provider repository for database access
    provider_repository: Arc<ProviderRepository>,
    /// Session manager for session tracking
    session_manager: Arc<SessionManager>,
    /// Rate limiter for request throttling
    rate_limiter: Arc<RateLimiter>,
    /// Database pool for model redirect queries
    db_pool: SqlitePool,
    /// Maximum number of retry attempts
    max_retries: usize,
    /// Base delay for exponential backoff (milliseconds)
    base_backoff_ms: u64,
}

impl RequestRouter {
    /// Create a new RequestRouter with all dependencies
    pub fn new(
        load_balancer: Arc<LoadBalancer>,
        circuit_breaker: Arc<CircuitBreaker>,
        provider_registry: Arc<ProviderRegistry>,
        provider_repository: Arc<ProviderRepository>,
        session_manager: Arc<SessionManager>,
        rate_limiter: Arc<RateLimiter>,
        db_pool: SqlitePool,
    ) -> Self {
        Self {
            load_balancer,
            circuit_breaker,
            provider_registry,
            provider_repository,
            session_manager,
            rate_limiter,
            db_pool,
            max_retries: 3,
            base_backoff_ms: 100,
        }
    }

    /// Load rate limit config for a provider
    async fn load_rate_limit_config(&self, provider_id: &str) -> Result<Option<RateLimitConfig>> {
        let row = sqlx::query(
            r#"
            SELECT rpm, tokens_per_5h, tokens_per_week, tokens_per_month, max_concurrent_sessions
            FROM rate_limit_configs
            WHERE provider_id = ?
            "#,
        )
        .bind(provider_id)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(Some(RateLimitConfig {
                rpm: row.try_get("rpm")?,
                tokens_per_5h: row.try_get("tokens_per_5h")?,
                tokens_per_week: row.try_get("tokens_per_week")?,
                tokens_per_month: row.try_get("tokens_per_month")?,
                max_concurrent_sessions: row.try_get("max_concurrent_sessions")?,
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a Backend with rate limit config loaded
    async fn create_backend(&self, provider: Provider) -> Result<Backend> {
        let rate_limit_config = self.load_rate_limit_config(&provider.id).await?;
        Ok(Backend::new(provider).with_rate_limit(rate_limit_config.unwrap_or_default()))
    }

    /// Select a backend provider for the given model and session
    /// Implements the complete routing decision flow:
    /// 1. Check session and prefer previous backend
    /// 2. Apply model redirect rules
    /// 3. Query available backends (excluding circuit-broken ones)
    /// 4. Select by priority and weight
    /// Requirements: 2.1, 2.2, 2.4, 9.2
    pub async fn select_backend(
        &self,
        model: &str,
        session_id: Option<&str>,
        decision_chain: &mut Vec<DecisionRecord>,
    ) -> Result<Backend> {
        tracing::info!(
            model = %model,
            session_id = ?session_id,
            "Starting backend selection"
        );

        let mut step_number = decision_chain.len() as u32 + 1;
        let mut target_model = model.to_string();

        // Step 1: Check if session exists and prefer previous backend
        // Requirements: 2.1, 6.2
        if let Some(sid) = session_id {
            if let Some(active_session) = self.session_manager.get_session(sid) {
                // Try to use the same backend as last time
                match self
                    .provider_repository
                    .get_by_id(&active_session.session.backend_id)
                    .await
                {
                    Ok(provider) if provider.enabled => {
                        // Check if circuit breaker allows this provider
                        if self.circuit_breaker.check_available(&provider.id).is_ok() {
                            tracing::info!(
                                provider_id = %provider.id,
                                provider_name = %provider.name,
                                session_id = %sid,
                                "Reusing backend from session"
                            );
                            decision_chain.push(
                                DecisionRecord::new(
                                    step_number,
                                    DecisionAction::Selected,
                                    format!("Reusing backend from session: {}", provider.name),
                                )
                                .with_provider(provider.id.clone()),
                            );
                            return self.create_backend(provider).await;
                        } else {
                            decision_chain.push(
                                DecisionRecord::new(
                                    step_number,
                                    DecisionAction::CircuitBreakerOpen,
                                    format!(
                                        "Previous backend {} has open circuit breaker",
                                        provider.name
                                    ),
                                )
                                .with_provider(provider.id.clone()),
                            );
                            step_number += 1;
                        }
                    }
                    Ok(provider) => {
                        decision_chain.push(
                            DecisionRecord::new(
                                step_number,
                                DecisionAction::Failed,
                                format!("Previous backend {} is disabled", provider.name),
                            )
                            .with_provider(provider.id.clone()),
                        );
                        step_number += 1;
                    }
                    Err(_) => {
                        decision_chain.push(DecisionRecord::new(
                            step_number,
                            DecisionAction::Failed,
                            "Previous backend not found".to_string(),
                        ));
                        step_number += 1;
                    }
                }
            }
        }

        // Step 2: Apply model redirect rules
        // Requirements: 9.2
        if let Ok(redirect) = self.get_model_redirect(&target_model).await {
            if redirect.enabled {
                tracing::info!(
                    source_model = %target_model,
                    target_model = %redirect.target_model,
                    target_provider_id = %redirect.target_provider_id,
                    "Applying model redirect rule"
                );
                decision_chain.push(
                    DecisionRecord::new(
                        step_number,
                        DecisionAction::Redirected,
                        format!(
                            "Model redirected from {} to {}",
                            target_model, redirect.target_model
                        ),
                    )
                    .with_provider(redirect.target_provider_id.clone()),
                );
                target_model = redirect.target_model;
                step_number += 1;

                // If redirect specifies a target provider, try to use it
                match self
                    .provider_repository
                    .get_by_id(&redirect.target_provider_id)
                    .await
                {
                    Ok(provider) if provider.enabled => {
                        if self.circuit_breaker.check_available(&provider.id).is_ok() {
                            decision_chain.push(
                                DecisionRecord::new(
                                    step_number,
                                    DecisionAction::Selected,
                                    format!("Using redirected provider: {}", provider.name),
                                )
                                .with_provider(provider.id.clone()),
                            );
                            return self.create_backend(provider).await;
                        } else {
                            decision_chain.push(
                                DecisionRecord::new(
                                    step_number,
                                    DecisionAction::CircuitBreakerOpen,
                                    format!(
                                        "Redirected provider {} has open circuit breaker",
                                        provider.name
                                    ),
                                )
                                .with_provider(provider.id.clone()),
                            );
                            step_number += 1;
                        }
                    }
                    Ok(provider) => {
                        decision_chain.push(
                            DecisionRecord::new(
                                step_number,
                                DecisionAction::Failed,
                                format!("Redirected provider {} is disabled", provider.name),
                            )
                            .with_provider(provider.id.clone()),
                        );
                        step_number += 1;
                    }
                    Err(_) => {
                        decision_chain.push(DecisionRecord::new(
                            step_number,
                            DecisionAction::Failed,
                            "Redirected provider not found".to_string(),
                        ));
                        step_number += 1;
                    }
                }
            }
        }

        // Step 3: Query available backends (excluding circuit-broken ones)
        // Requirements: 2.1, 2.3
        let providers = self.provider_repository.get_enabled().await?;

        if providers.is_empty() {
            decision_chain.push(DecisionRecord::new(
                step_number,
                DecisionAction::Failed,
                "No enabled providers available".to_string(),
            ));
            return Err(RouterError::NoAvailableBackends(target_model).into());
        }

        // Filter out providers with open circuit breakers
        let available_providers: Vec<Provider> = providers
            .into_iter()
            .filter(|p| {
                let is_available = self.circuit_breaker.check_available(&p.id).is_ok();
                if !is_available {
                    tracing::debug!("Excluding provider {} due to open circuit breaker", p.name);
                }
                is_available
            })
            .collect();

        if available_providers.is_empty() {
            decision_chain.push(DecisionRecord::new(
                step_number,
                DecisionAction::CircuitBreakerOpen,
                "All providers have open circuit breakers".to_string(),
            ));
            return Err(AppError::CircuitBreakerOpen(
                "All providers have open circuit breakers".to_string(),
            ));
        }

        // Step 4: Select by priority and weight using load balancer
        // Requirements: 2.1, 2.2
        // Note: We create backends without rate limit config for selection,
        // then load the config for the selected backend
        let backends: Vec<Backend> = available_providers.into_iter().map(Backend::new).collect();

        let selected_provider = self
            .load_balancer
            .select_backend(backends, None)
            .ok_or_else(|| {
                decision_chain.push(DecisionRecord::new(
                    step_number,
                    DecisionAction::Failed,
                    "Load balancer failed to select backend".to_string(),
                ));
                RouterError::SelectionFailed("Load balancer returned no backend".to_string())
            })?
            .provider;

        let backend = self.create_backend(selected_provider.clone()).await?;

        decision_chain.push(
            DecisionRecord::new(
                step_number,
                DecisionAction::Selected,
                format!(
                    "Selected backend {} (priority: {}, weight: {})",
                    selected_provider.name, selected_provider.priority, selected_provider.weight
                ),
            )
            .with_provider(selected_provider.id.clone()),
        );

        Ok(backend)
    }

    /// Route a request to an appropriate backend with retry and failover logic
    /// Requirements: 2.4, 11.1
    pub async fn route_request(
        &self,
        request: &ChatCompletionRequest,
        session_id: Option<String>,
    ) -> Result<(ChatCompletionResponse, String, Vec<DecisionRecord>)> {
        let mut decision_chain = Vec::new();
        let start_time = Instant::now();

        // Get session ID reference for backend selection
        let session_id_ref = session_id.as_deref();

        // Attempt to route with retries
        let mut last_error: Option<AppError> = None;
        let mut excluded_backends: Vec<String> = Vec::new();

        for attempt in 0..=self.max_retries {
            // Select backend
            let backend = match self
                .select_backend_with_exclusions(
                    &request.model,
                    session_id_ref,
                    &excluded_backends,
                    &mut decision_chain,
                )
                .await
            {
                Ok(b) => b,
                Err(e) => {
                    last_error = Some(e);
                    break;
                }
            };

            let provider_id = backend.provider.id.clone();

            // Check rate limits
            // Estimate tokens (rough estimate: 4 chars per token)
            let estimated_tokens = request
                .messages
                .iter()
                .map(|m| m.content.as_ref().map(|c| c.len() / 4).unwrap_or(0) as u64)
                .sum::<u64>()
                + request.max_tokens.unwrap_or(1000) as u64;

            if let Err(e) = self
                .rate_limiter
                .check_limit(
                    &provider_id,
                    estimated_tokens,
                    &backend.rate_limit_config.clone().unwrap_or_default(),
                )
                .await
            {
                decision_chain.push(
                    DecisionRecord::new(
                        decision_chain.len() as u32 + 1,
                        DecisionAction::RateLimited,
                        format!("Rate limit exceeded: {}", e),
                    )
                    .with_provider(provider_id.clone()),
                );

                // Don't retry on rate limit, return error immediately
                return Err(e.into());
            }

            // Get provider adapter
            let adapter = self
                .provider_registry
                .get_adapter(&backend.provider.provider_type)?;

            // Send request to provider
            match adapter.send_request(request, &backend.provider).await {
                Ok(response) => {
                    let latency_ms = start_time.elapsed().as_millis() as u64;

                    // Record success in circuit breaker
                    self.circuit_breaker.record_success(&provider_id);

                    // Consume rate limit tokens
                    let actual_tokens = response.usage.total_tokens as u64;
                    if let Err(e) = self.rate_limiter.consume(&provider_id, actual_tokens).await {
                        tracing::warn!("Failed to consume rate limit tokens: {}", e);
                    }

                    // Create or update session
                    let final_session_id = if let Some(sid) = session_id {
                        self.session_manager.update_activity(&sid).await?;
                        sid
                    } else {
                        let new_session = self
                            .session_manager
                            .get_or_create_session(None, provider_id.clone())
                            .await?;
                        new_session.session.id
                    };

                    // Add decision records to session
                    for decision in &decision_chain {
                        self.session_manager
                            .add_decision(&final_session_id, decision.clone());
                    }

                    tracing::info!(
                        "Request routed successfully to {} in {}ms",
                        backend.provider.name,
                        latency_ms
                    );

                    return Ok((response, final_session_id, decision_chain));
                }
                Err(e) => {
                    // Record failure in circuit breaker
                    self.circuit_breaker.record_failure(&provider_id);

                    decision_chain.push(
                        DecisionRecord::new(
                            decision_chain.len() as u32 + 1,
                            DecisionAction::Failed,
                            format!("Request failed: {}", e),
                        )
                        .with_provider(provider_id.clone()),
                    );

                    // Exclude this backend from future attempts
                    excluded_backends.push(provider_id.clone());

                    last_error = Some(e);

                    // Check if we should retry
                    if attempt < self.max_retries {
                        // Exponential backoff
                        let backoff_ms = self.base_backoff_ms * 2_u64.pow(attempt as u32);
                        tracing::warn!(
                            "Request to {} failed, retrying in {}ms (attempt {}/{})",
                            backend.provider.name,
                            backoff_ms,
                            attempt + 1,
                            self.max_retries
                        );

                        decision_chain.push(DecisionRecord::new(
                            decision_chain.len() as u32 + 1,
                            DecisionAction::Retried,
                            format!("Retrying after {}ms backoff", backoff_ms),
                        ));

                        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    } else {
                        tracing::error!(
                            "All retry attempts exhausted for request to model {}",
                            request.model
                        );
                    }
                }
            }
        }

        // All retries failed
        Err(last_error.unwrap_or_else(|| RouterError::AllBackendsFailed(self.max_retries).into()))
    }

    /// Select backend with exclusion list for failover
    async fn select_backend_with_exclusions(
        &self,
        model: &str,
        session_id: Option<&str>,
        excluded_backends: &[String],
        decision_chain: &mut Vec<DecisionRecord>,
    ) -> Result<Backend> {
        let mut step_number = decision_chain.len() as u32 + 1;
        let mut target_model = model.to_string();

        // Step 1: Check if session exists and prefer previous backend (if not excluded)
        if let Some(sid) = session_id {
            if let Some(active_session) = self.session_manager.get_session(sid) {
                if !excluded_backends.contains(&active_session.session.backend_id) {
                    match self
                        .provider_repository
                        .get_by_id(&active_session.session.backend_id)
                        .await
                    {
                        Ok(provider) if provider.enabled => {
                            if self.circuit_breaker.check_available(&provider.id).is_ok() {
                                decision_chain.push(
                                    DecisionRecord::new(
                                        step_number,
                                        DecisionAction::Selected,
                                        format!("Reusing backend from session: {}", provider.name),
                                    )
                                    .with_provider(provider.id.clone()),
                                );
                                return self.create_backend(provider).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Step 2: Apply model redirect rules
        if let Ok(redirect) = self.get_model_redirect(&target_model).await {
            if redirect.enabled {
                decision_chain.push(
                    DecisionRecord::new(
                        step_number,
                        DecisionAction::Redirected,
                        format!(
                            "Model redirected from {} to {}",
                            target_model, redirect.target_model
                        ),
                    )
                    .with_provider(redirect.target_provider_id.clone()),
                );
                target_model = redirect.target_model;
                step_number += 1;

                if !excluded_backends.contains(&redirect.target_provider_id) {
                    match self
                        .provider_repository
                        .get_by_id(&redirect.target_provider_id)
                        .await
                    {
                        Ok(provider) if provider.enabled => {
                            if self.circuit_breaker.check_available(&provider.id).is_ok() {
                                decision_chain.push(
                                    DecisionRecord::new(
                                        step_number,
                                        DecisionAction::Selected,
                                        format!("Using redirected provider: {}", provider.name),
                                    )
                                    .with_provider(provider.id.clone()),
                                );
                                return self.create_backend(provider).await;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Step 3: Query available backends (excluding circuit-broken and excluded ones)
        let providers = self.provider_repository.get_enabled().await?;

        let available_providers: Vec<Provider> = providers
            .into_iter()
            .filter(|p| {
                !excluded_backends.contains(&p.id)
                    && self.circuit_breaker.check_available(&p.id).is_ok()
            })
            .collect();

        if available_providers.is_empty() {
            decision_chain.push(DecisionRecord::new(
                step_number,
                DecisionAction::Failed,
                "No available backends after exclusions".to_string(),
            ));
            return Err(RouterError::NoAvailableBackends(target_model).into());
        }

        // Step 4: Select by priority and weight
        let backends: Vec<Backend> = available_providers.into_iter().map(Backend::new).collect();

        let selected_provider = self
            .load_balancer
            .select_backend(backends, None)
            .ok_or_else(|| {
                RouterError::SelectionFailed("Load balancer returned no backend".to_string())
            })?
            .provider;

        let backend = self.create_backend(selected_provider.clone()).await?;

        decision_chain.push(
            DecisionRecord::new(
                step_number,
                DecisionAction::Selected,
                format!(
                    "Selected backend {} (priority: {}, weight: {})",
                    selected_provider.name, selected_provider.priority, selected_provider.weight
                ),
            )
            .with_provider(selected_provider.id.clone()),
        );

        Ok(backend)
    }

    /// Get model redirect rule for a given source model
    async fn get_model_redirect(&self, source_model: &str) -> Result<ModelRedirect> {
        let row = sqlx::query(
            r#"
            SELECT id, source_model, target_model, target_provider_id, enabled, created_at
            FROM model_redirects
            WHERE source_model = ? AND enabled = TRUE
            LIMIT 1
            "#,
        )
        .bind(source_model)
        .fetch_optional(&self.db_pool)
        .await?;

        if let Some(row) = row {
            use sqlx::Row;
            Ok(ModelRedirect {
                id: row.try_get("id")?,
                source_model: row.try_get("source_model")?,
                target_model: row.try_get("target_model")?,
                target_provider_id: row.try_get("target_provider_id")?,
                enabled: row.try_get("enabled")?,
                created_at: row.try_get("created_at")?,
            })
        } else {
            Err(AppError::NotFound(format!(
                "No redirect rule found for model: {}",
                source_model
            )))
        }
    }

    /// Build an HTTP client with proxy support for a provider
    /// Requirements: 8.1, 8.2, 8.3, 8.5
    pub fn build_client_with_proxy(provider: &Provider) -> Result<Client> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(10);

        // Configure proxy if specified
        if let Some(proxy_config) = &provider.proxy {
            tracing::debug!(
                "Configuring {} proxy for provider {}: {}",
                proxy_config.proxy_type,
                provider.name,
                proxy_config.url
            );

            // Create proxy based on type
            let proxy = match proxy_config.proxy_type {
                ProxyType::Http | ProxyType::Https => {
                    // For HTTP/HTTPS, use all() to proxy all traffic
                    reqwest::Proxy::all(&proxy_config.url).map_err(|e| {
                        ProviderError::ProxyError(format!("Invalid proxy URL: {}", e))
                    })?
                }
                ProxyType::Socks5 => {
                    // For SOCKS5, use all() as well (reqwest supports SOCKS5)
                    reqwest::Proxy::all(&proxy_config.url).map_err(|e| {
                        ProviderError::ProxyError(format!("Invalid SOCKS5 proxy URL: {}", e))
                    })?
                }
            };

            // Add authentication if provided
            let proxy = if let Some(auth) = &proxy_config.auth {
                tracing::debug!(
                    "Adding proxy authentication for provider {} (username: {})",
                    provider.name,
                    auth.username
                );
                proxy.basic_auth(&auth.username, &auth.password)
            } else {
                proxy
            };

            builder = builder.proxy(proxy);
        }

        builder.build().map_err(|e| {
            ProviderError::ConnectionFailed(format!("Failed to build HTTP client: {}", e)).into()
        })
    }

    /// Validate proxy configuration for a provider
    /// Requirements: 8.4
    pub async fn validate_proxy(provider: &Provider) -> Result<()> {
        if let Some(proxy_config) = &provider.proxy {
            // Build client with proxy
            let client = Self::build_client_with_proxy(provider)?;

            // Try a simple request to validate connectivity
            // Use a reliable endpoint for testing
            let test_url = match proxy_config.proxy_type {
                ProxyType::Http | ProxyType::Https | ProxyType::Socks5 => "https://www.google.com",
            };

            match client
                .get(test_url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) => {
                    if response.status().is_success() || response.status().is_redirection() {
                        tracing::info!(
                            "Proxy validation successful for provider {}: {} ({})",
                            provider.name,
                            proxy_config.url,
                            proxy_config.proxy_type
                        );
                        Ok(())
                    } else {
                        Err(ProviderError::ProxyError(format!(
                            "Proxy test request failed with status: {}",
                            response.status()
                        ))
                        .into())
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Proxy validation failed for provider {}: {}",
                        provider.name,
                        e
                    );
                    Err(
                        ProviderError::ProxyError(format!("Proxy connection test failed: {}", e))
                            .into(),
                    )
                }
            }
        } else {
            // No proxy configured, validation passes
            Ok(())
        }
    }
}
