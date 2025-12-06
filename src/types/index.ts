// Type definitions matching Rust models

export type ProviderType = 'anthropic' | 'openai' | 'gemini' | 'custom'

export type ProxyType = 'http' | 'https' | 'socks5'

export interface ProxyAuth {
  username: string
  password: string
}

export interface ProxyConfig {
  proxy_type: ProxyType
  url: string
  auth?: ProxyAuth
}

export interface Provider {
  id: string
  name: string
  provider_type: ProviderType
  endpoint: string
  api_key: string
  priority: number
  weight: number
  enabled: boolean
  proxy?: ProxyConfig
  created_at: number
  updated_at: number
}

export interface DashboardMetrics {
  qps: number
  error_rate: number
  active_sessions: number
  total_requests_1h: number
  total_tokens_1h: number
  avg_latency_ms: number
}

export interface SessionInfo {
  id: string
  backend_id: string
  provider_name: string
  created_at: number
  last_activity: number
  duration_seconds: number
}

export interface Message {
  role: string
  content: string
}

export interface ToolFunction {
  name: string
  description?: string
  parameters: Record<string, unknown>
}

export interface Tool {
  type: string
  function: ToolFunction
}

export interface ChatCompletionRequest {
  model: string
  messages: Message[]
  temperature?: number
  max_tokens?: number
  stream?: boolean
  tools?: Tool[]
}

export interface Usage {
  prompt_tokens: number
  completion_tokens: number
  total_tokens: number
}

export interface Choice {
  index: number
  message: Message
  finish_reason?: string
}

export interface ChatCompletionResponse {
  id: string
  model: string
  choices: Choice[]
  usage: Usage
}

export interface CircuitBreakerStatus {
  provider_id: string
  provider_name: string
  state: 'closed' | 'open' | 'half_open'
  failure_count: number
  success_count: number
  last_failure?: number
  opened_at?: number
}

export interface ConsumptionRanking {
  user_id: string
  request_count: number
  total_tokens: number
  cost_usd: number
}

export type DecisionAction =
  | 'selected'
  | 'failed'
  | 'retried'
  | 'redirected'
  | 'circuit_breaker_open'
  | 'rate_limited'

export interface DecisionRecord {
  step_number: number
  action: DecisionAction
  provider_id?: string
  reason: string
  timestamp: number
}

export interface ModelPricing {
  id: string
  model_name: string
  provider: string
  input_cost_per_1m: number
  output_cost_per_1m: number
  last_updated: number
}

export interface ServerConfig {
  host: string
  port: number
  enable_tls: boolean
}

export interface DatabaseConfig {
  path: string
}

export interface CircuitBreakerConfig {
  failure_threshold: number
  window_duration_secs: number
  cooldown_duration_secs: number
}

export interface SessionConfig {
  ttl_minutes: number
  cleanup_interval_secs: number
}

export interface LoggingConfig {
  level: string
  file?: string
}

export interface AppConfig {
  server: ServerConfig
  database: DatabaseConfig
  circuit_breaker: CircuitBreakerConfig
  session: SessionConfig
  logging: LoggingConfig
}

export interface ModelRedirect {
  id: string
  source_model: string
  target_model: string
  target_provider_id: string
  enabled: boolean
  created_at: number
}
