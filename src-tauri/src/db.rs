// Database module
use crate::error::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(path: &str) -> Result<Self> {
        let options = SqliteConnectOptions::from_str(path)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn migrate(&self) -> Result<()> {
        // Create all tables
        self.create_providers_table().await?;
        self.create_rate_limit_configs_table().await?;
        self.create_model_redirects_table().await?;
        self.create_sessions_table().await?;
        self.create_usage_logs_table().await?;
        self.create_decision_chains_table().await?;
        self.create_model_pricing_table().await?;
        self.create_circuit_breaker_states_table().await?;

        // Create indexes
        self.create_indexes().await?;

        Ok(())
    }

    async fn create_providers_table(&self) -> Result<()> {
        sqlx::query(
            r#"
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
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_rate_limit_configs_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS rate_limit_configs (
                provider_id TEXT PRIMARY KEY,
                rpm INTEGER,
                tokens_per_5h INTEGER,
                tokens_per_week INTEGER,
                tokens_per_month INTEGER,
                max_concurrent_sessions INTEGER,
                FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_model_redirects_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_redirects (
                id TEXT PRIMARY KEY,
                source_model TEXT NOT NULL,
                target_model TEXT NOT NULL,
                target_provider_id TEXT NOT NULL,
                enabled BOOLEAN DEFAULT TRUE,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (target_provider_id) REFERENCES providers(id) ON DELETE CASCADE,
                UNIQUE(source_model)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_sessions_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                backend_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                last_activity INTEGER NOT NULL,
                expired BOOLEAN DEFAULT FALSE,
                FOREIGN KEY (backend_id) REFERENCES providers(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_usage_logs_table(&self) -> Result<()> {
        sqlx::query(
            r#"
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
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_decision_chains_table(&self) -> Result<()> {
        sqlx::query(
            r#"
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
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_model_pricing_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS model_pricing (
                id TEXT PRIMARY KEY,
                model_name TEXT NOT NULL UNIQUE,
                provider TEXT NOT NULL,
                input_cost_per_1m REAL NOT NULL,
                output_cost_per_1m REAL NOT NULL,
                last_updated INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_circuit_breaker_states_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS circuit_breaker_states (
                provider_id TEXT PRIMARY KEY,
                state TEXT NOT NULL,
                failure_count INTEGER DEFAULT 0,
                success_count INTEGER DEFAULT 0,
                last_failure INTEGER,
                opened_at INTEGER,
                FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_indexes(&self) -> Result<()> {
        // Index for usage_logs queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_logs_created_at 
            ON usage_logs(created_at)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_usage_logs_provider_id 
            ON usage_logs(provider_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Index for sessions queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_last_activity 
            ON sessions(last_activity)
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_expired 
            ON sessions(expired)
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Index for decision_chains queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_decision_chains_usage_log_id 
            ON decision_chains(usage_log_id)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
