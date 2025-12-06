use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Provider Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub provider_type: ProviderType,
    pub endpoint: String,
    pub api_key: String,
    pub priority: i32,
    pub weight: i32,
    pub enabled: bool,
    pub proxy: Option<ProxyConfig>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Anthropic,
    OpenAI,
    Gemini,
    Custom,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Gemini => write!(f, "gemini"),
            ProviderType::Custom => write!(f, "custom"),
        }
    }
}

// ============================================================================
// Proxy Configuration Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    pub proxy_type: ProxyType,
    pub url: String,
    pub auth: Option<ProxyAuth>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProxyType {
    Http,
    Https,
    Socks5,
}

impl std::fmt::Display for ProxyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProxyType::Http => write!(f, "http"),
            ProxyType::Https => write!(f, "https"),
            ProxyType::Socks5 => write!(f, "socks5"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyAuth {
    pub username: String,
    pub password: String,
}

// ============================================================================
// Chat Completion Request/Response Models (OpenAI Compatible)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,
    #[serde(default)]
    pub stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Specific {
        r#type: String,
        function: ToolChoiceFunction,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoiceFunction {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<TokenDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<TokenDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

// ============================================================================
// Streaming Response Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: Delta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Delta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

// ============================================================================
// Rate Limiting Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub rpm: Option<u32>,
    pub tokens_per_5h: Option<u64>,
    pub tokens_per_week: Option<u64>,
    pub tokens_per_month: Option<u64>,
    pub max_concurrent_sessions: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            rpm: None,
            tokens_per_5h: None,
            tokens_per_week: None,
            tokens_per_month: None,
            max_concurrent_sessions: None,
        }
    }
}

// ============================================================================
// Session Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub backend_id: String,
    pub created_at: i64,
    pub last_activity: i64,
    pub expired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub backend_id: String,
    pub provider_name: String,
    pub created_at: i64,
    pub last_activity: i64,
    pub duration_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecord {
    pub step_number: u32,
    pub action: DecisionAction,
    pub provider_id: Option<String>,
    pub reason: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DecisionAction {
    Selected,
    Failed,
    Retried,
    Redirected,
    CircuitBreakerOpen,
    RateLimited,
}

impl std::fmt::Display for DecisionAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionAction::Selected => write!(f, "selected"),
            DecisionAction::Failed => write!(f, "failed"),
            DecisionAction::Retried => write!(f, "retried"),
            DecisionAction::Redirected => write!(f, "redirected"),
            DecisionAction::CircuitBreakerOpen => write!(f, "circuit_breaker_open"),
            DecisionAction::RateLimited => write!(f, "rate_limited"),
        }
    }
}

// ============================================================================
// Usage and Monitoring Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLog {
    pub id: String,
    pub session_id: Option<String>,
    pub provider_id: String,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub cost_usd: f64,
    pub latency_ms: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UsageStatus {
    Success,
    Failed,
    RateLimited,
    CircuitBreakerOpen,
}

impl std::fmt::Display for UsageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UsageStatus::Success => write!(f, "success"),
            UsageStatus::Failed => write!(f, "failed"),
            UsageStatus::RateLimited => write!(f, "rate_limited"),
            UsageStatus::CircuitBreakerOpen => write!(f, "circuit_breaker_open"),
        }
    }
}

impl From<UsageStatus> for String {
    fn from(status: UsageStatus) -> Self {
        status.to_string()
    }
}

impl std::str::FromStr for UsageStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "success" => Ok(UsageStatus::Success),
            "failed" => Ok(UsageStatus::Failed),
            "rate_limited" => Ok(UsageStatus::RateLimited),
            "circuit_breaker_open" => Ok(UsageStatus::CircuitBreakerOpen),
            _ => Err(format!("Invalid usage status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMetrics {
    pub qps: f64,
    pub error_rate: f64,
    pub active_sessions: u32,
    pub total_requests_1h: u64,
    pub total_tokens_1h: u64,
    pub avg_latency_ms: f64,
}

// ============================================================================
// Model Pricing Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub id: String,
    pub model_name: String,
    pub provider: String,
    pub input_cost_per_1m: f64,
    pub output_cost_per_1m: f64,
    pub last_updated: i64,
}

// ============================================================================
// Model Redirect Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRedirect {
    pub id: String,
    pub source_model: String,
    pub target_model: String,
    pub target_provider_id: String,
    pub enabled: bool,
    pub created_at: i64,
}

// ============================================================================
// Circuit Breaker Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half_open"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerState {
    pub provider_id: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure: Option<i64>,
    pub opened_at: Option<i64>,
}

// ============================================================================
// Backend Selection Models
// ============================================================================

#[derive(Debug, Clone)]
pub struct Backend {
    pub provider: Provider,
    pub rate_limit_config: Option<RateLimitConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendMetrics {
    pub provider_id: String,
    pub total_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub last_success: Option<i64>,
    pub last_failure: Option<i64>,
}

// ============================================================================
// Load Balancing Models
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadBalancingStrategy {
    WeightedRoundRobin,
    PriorityBased,
    LeastLatency,
}

impl Default for LoadBalancingStrategy {
    fn default() -> Self {
        Self::PriorityBased
    }
}

// ============================================================================
// Model Implementations
// ============================================================================

impl Provider {
    pub fn new(
        name: String,
        provider_type: ProviderType,
        endpoint: String,
        api_key: String,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            provider_type,
            endpoint,
            api_key,
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_weight(mut self, weight: i32) -> Self {
        self.weight = weight;
        self
    }

    pub fn with_proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

impl ChatCompletionResponse {
    pub fn new(id: String, model: String, choices: Vec<Choice>, usage: Usage) -> Self {
        Self {
            id,
            object: "chat.completion".to_string(),
            created: chrono::Utc::now().timestamp(),
            model,
            choices,
            usage,
            system_fingerprint: None,
        }
    }
}

impl ChatCompletionChunk {
    pub fn new(id: String, model: String, choices: Vec<ChunkChoice>) -> Self {
        Self {
            id,
            object: "chat.completion.chunk".to_string(),
            created: chrono::Utc::now().timestamp(),
            model,
            choices,
            system_fingerprint: None,
        }
    }
}

impl Usage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }
    }

    pub fn with_reasoning_tokens(mut self, reasoning_tokens: u32) -> Self {
        self.completion_tokens_details = Some(TokenDetails {
            cached_tokens: None,
            reasoning_tokens: Some(reasoning_tokens),
        });
        self
    }
}

impl UsageLog {
    pub fn new(
        session_id: Option<String>,
        provider_id: String,
        model: String,
        usage: &Usage,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            session_id,
            provider_id,
            model,
            prompt_tokens: usage.prompt_tokens as i32,
            completion_tokens: usage.completion_tokens as i32,
            total_tokens: usage.total_tokens as i32,
            cost_usd: 0.0,
            latency_ms: None,
            status: UsageStatus::Success.to_string(),
            error_message: None,
            created_at: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_cost(mut self, cost_usd: f64) -> Self {
        self.cost_usd = cost_usd;
        self
    }

    pub fn with_latency(mut self, latency_ms: i32) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }

    pub fn with_status(mut self, status: UsageStatus) -> Self {
        self.status = status.to_string();
        self
    }

    pub fn with_error(mut self, error_message: String) -> Self {
        self.error_message = Some(error_message);
        self.status = UsageStatus::Failed.to_string();
        self
    }

    pub fn get_status(&self) -> Result<UsageStatus, String> {
        self.status.parse()
    }
}

impl Session {
    pub fn new(backend_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            backend_id,
            created_at: now,
            last_activity: now,
            expired: false,
        }
    }

    pub fn is_expired(&self, ttl_seconds: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.last_activity > ttl_seconds
    }

    pub fn update_activity(&mut self) {
        self.last_activity = chrono::Utc::now().timestamp();
    }
}

impl DecisionRecord {
    pub fn new(step_number: u32, action: DecisionAction, reason: String) -> Self {
        Self {
            step_number,
            action,
            provider_id: None,
            reason,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    pub fn with_provider(mut self, provider_id: String) -> Self {
        self.provider_id = Some(provider_id);
        self
    }
}

impl ModelRedirect {
    pub fn new(source_model: String, target_model: String, target_provider_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source_model,
            target_model,
            target_provider_id,
            enabled: true,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

impl Backend {
    pub fn new(provider: Provider) -> Self {
        Self {
            provider,
            rate_limit_config: None,
        }
    }

    pub fn with_rate_limit(mut self, config: RateLimitConfig) -> Self {
        self.rate_limit_config = Some(config);
        self
    }
}
