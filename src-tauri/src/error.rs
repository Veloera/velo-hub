use thiserror::Error;

// ============================================================================
// Top-level Application Error
// ============================================================================

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Router error: {0}")]
    Router(#[from] RouterError),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(#[from] RateLimitError),

    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Circuit breaker open for provider: {0}")]
    CircuitBreakerOpen(String),

    #[error("Crypto error: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, AppError>;

// Convert AppError to String for Tauri commands
impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}

// ============================================================================
// Router Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum RouterError {
    #[error("No available backends for model: {0}")]
    NoAvailableBackends(String),

    #[error("All backends failed after {0} retries")]
    AllBackendsFailed(usize),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Backend selection failed: {0}")]
    SelectionFailed(String),

    #[error("Request forwarding failed: {0}")]
    ForwardingFailed(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Model redirect error: {0}")]
    RedirectError(String),

    #[error("Invalid backend configuration: {0}")]
    InvalidBackend(String),
}

// ============================================================================
// Rate Limit Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum RateLimitError {
    #[error("RPM limit exceeded for provider {provider_id}, retry after {retry_after_secs}s")]
    RpmExceeded {
        provider_id: String,
        retry_after_secs: u64,
    },

    #[error("Token quota exceeded for provider {provider_id} in {window} window")]
    TokenQuotaExceeded { provider_id: String, window: String },

    #[error("Max concurrent sessions reached for provider: {0}")]
    ConcurrencyExceeded(String),

    #[error("Rate limiter storage error: {0}")]
    StorageError(String),

    #[error("Rate limit check failed: {0}")]
    CheckFailed(String),
}

// ============================================================================
// Provider Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider not found: {0}")]
    NotFound(String),

    #[error("Provider is disabled: {0}")]
    Disabled(String),

    #[error("Invalid provider configuration: {0}")]
    InvalidConfig(String),

    #[error("Provider API error: {0}")]
    ApiError(String),

    #[error("Provider authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Provider request timeout: {0}")]
    Timeout(String),

    #[error("Provider connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Unsupported provider type: {0}")]
    UnsupportedType(String),

    #[error("Format conversion error: {0}")]
    FormatConversion(String),

    #[error("Provider response error: {0}")]
    ResponseError(String),

    #[error("Proxy configuration error: {0}")]
    ProxyError(String),
}

// ============================================================================
// Crypto Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    #[error("Keyring error: {0}")]
    KeyringError(String),

    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
}

// ============================================================================
// Validation Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Invalid field {field}: {message}")]
    InvalidField { field: String, message: String },

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Validation failed: {0}")]
    Failed(String),
}

// ============================================================================
// Circuit Breaker Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum CircuitBreakerError {
    #[error("Circuit breaker is open for provider: {0}")]
    Open(String),

    #[error("Circuit breaker state transition failed: {0}")]
    StateTransitionFailed(String),

    #[error("Circuit breaker configuration error: {0}")]
    ConfigError(String),
}

// ============================================================================
// Session Errors
// ============================================================================

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Session expired: {0}")]
    Expired(String),

    #[error("Session creation failed: {0}")]
    CreationFailed(String),

    #[error("Session update failed: {0}")]
    UpdateFailed(String),
}
