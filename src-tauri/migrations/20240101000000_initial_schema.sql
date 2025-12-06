-- Initial database schema for LLM API Gateway

-- providers table
CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    api_key_encrypted BLOB NOT NULL,
    priority INTEGER DEFAULT 0,
    weight INTEGER DEFAULT 1,
    enabled BOOLEAN DEFAULT TRUE,
    proxy_type TEXT,
    proxy_url TEXT,
    proxy_auth_encrypted BLOB,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_providers_enabled ON providers(enabled);
CREATE INDEX IF NOT EXISTS idx_providers_priority ON providers(priority);

-- rate_limit_configs table
CREATE TABLE IF NOT EXISTS rate_limit_configs (
    provider_id TEXT PRIMARY KEY,
    rpm INTEGER,
    tokens_per_5h INTEGER,
    tokens_per_week INTEGER,
    tokens_per_month INTEGER,
    max_concurrent_sessions INTEGER,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

-- model_redirects table
CREATE TABLE IF NOT EXISTS model_redirects (
    id TEXT PRIMARY KEY,
    source_model TEXT NOT NULL,
    target_model TEXT NOT NULL,
    target_provider_id TEXT NOT NULL,
    enabled BOOLEAN DEFAULT TRUE,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (target_provider_id) REFERENCES providers(id) ON DELETE CASCADE,
    UNIQUE(source_model)
);

CREATE INDEX IF NOT EXISTS idx_model_redirects_source ON model_redirects(source_model);
CREATE INDEX IF NOT EXISTS idx_model_redirects_enabled ON model_redirects(enabled);

-- sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    backend_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    last_activity INTEGER NOT NULL,
    expired BOOLEAN DEFAULT FALSE,
    FOREIGN KEY (backend_id) REFERENCES providers(id)
);

CREATE INDEX IF NOT EXISTS idx_sessions_last_activity ON sessions(last_activity);
CREATE INDEX IF NOT EXISTS idx_sessions_expired ON sessions(expired);
CREATE INDEX IF NOT EXISTS idx_sessions_backend_id ON sessions(backend_id);

-- usage_logs table
CREATE TABLE IF NOT EXISTS usage_logs (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    provider_id TEXT NOT NULL,
    model TEXT NOT NULL,
    prompt_tokens INTEGER DEFAULT 0,
    completion_tokens INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    cost_usd REAL DEFAULT 0.0,
    latency_ms INTEGER,
    status TEXT NOT NULL,
    error_message TEXT,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id),
    FOREIGN KEY (provider_id) REFERENCES providers(id)
);

CREATE INDEX IF NOT EXISTS idx_usage_logs_created_at ON usage_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_logs_provider_id ON usage_logs(provider_id);
CREATE INDEX IF NOT EXISTS idx_usage_logs_session_id ON usage_logs(session_id);
CREATE INDEX IF NOT EXISTS idx_usage_logs_status ON usage_logs(status);

-- decision_chains table
CREATE TABLE IF NOT EXISTS decision_chains (
    id TEXT PRIMARY KEY,
    usage_log_id TEXT NOT NULL,
    step_number INTEGER NOT NULL,
    action TEXT NOT NULL,
    provider_id TEXT,
    reason TEXT,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (usage_log_id) REFERENCES usage_logs(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES providers(id)
);

CREATE INDEX IF NOT EXISTS idx_decision_chains_usage_log ON decision_chains(usage_log_id);
CREATE INDEX IF NOT EXISTS idx_decision_chains_provider ON decision_chains(provider_id);

-- model_pricing table
CREATE TABLE IF NOT EXISTS model_pricing (
    id TEXT PRIMARY KEY,
    model_name TEXT NOT NULL UNIQUE,
    provider TEXT NOT NULL,
    input_cost_per_1m REAL NOT NULL,
    output_cost_per_1m REAL NOT NULL,
    last_updated INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_model_pricing_model_name ON model_pricing(model_name);
CREATE INDEX IF NOT EXISTS idx_model_pricing_provider ON model_pricing(provider);

-- circuit_breaker_states table
CREATE TABLE IF NOT EXISTS circuit_breaker_states (
    provider_id TEXT PRIMARY KEY,
    state TEXT NOT NULL,
    failure_count INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    last_failure INTEGER,
    opened_at INTEGER,
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

-- global_config table
CREATE TABLE IF NOT EXISTS global_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    config_json TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
