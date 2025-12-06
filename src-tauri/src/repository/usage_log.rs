use crate::error::{AppError, Result};
use crate::models::UsageLog;
use sqlx::SqlitePool;

pub struct UsageLogRepository {
    pool: SqlitePool,
}

impl UsageLogRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, log: &UsageLog) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO usage_logs (
                id, session_id, provider_id, model, prompt_tokens, completion_tokens,
                total_tokens, cost_usd, latency_ms, status, error_message, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&log.id)
        .bind(&log.session_id)
        .bind(&log.provider_id)
        .bind(&log.model)
        .bind(log.prompt_tokens)
        .bind(log.completion_tokens)
        .bind(log.total_tokens)
        .bind(log.cost_usd)
        .bind(log.latency_ms)
        .bind(&log.status)
        .bind(&log.error_message)
        .bind(log.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<UsageLog> {
        let row = sqlx::query(
            r#"
            SELECT id, session_id, provider_id, model, prompt_tokens, completion_tokens,
                   total_tokens, cost_usd, latency_ms, status, error_message, created_at
            FROM usage_logs
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Usage log with id {} not found", id)))?;

        self.row_to_usage_log(row)
    }

    pub async fn get_by_session(&self, session_id: &str) -> Result<Vec<UsageLog>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, provider_id, model, prompt_tokens, completion_tokens,
                   total_tokens, cost_usd, latency_ms, status, error_message, created_at
            FROM usage_logs
            WHERE session_id = ?
            ORDER BY created_at ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_usage_log(row))
            .collect()
    }

    pub async fn get_by_provider(&self, provider_id: &str, limit: i64) -> Result<Vec<UsageLog>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, provider_id, model, prompt_tokens, completion_tokens,
                   total_tokens, cost_usd, latency_ms, status, error_message, created_at
            FROM usage_logs
            WHERE provider_id = ?
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(provider_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_usage_log(row))
            .collect()
    }

    pub async fn get_recent(&self, limit: i64) -> Result<Vec<UsageLog>> {
        let rows = sqlx::query(
            r#"
            SELECT id, session_id, provider_id, model, prompt_tokens, completion_tokens,
                   total_tokens, cost_usd, latency_ms, status, error_message, created_at
            FROM usage_logs
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_usage_log(row))
            .collect()
    }

    pub async fn get_stats_by_provider(
        &self,
        provider_id: &str,
        start_time: i64,
        end_time: i64,
    ) -> Result<(i64, i64, f64)> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as request_count,
                COALESCE(SUM(total_tokens), 0) as total_tokens,
                COALESCE(SUM(cost_usd), 0.0) as total_cost
            FROM usage_logs
            WHERE provider_id = ? AND created_at BETWEEN ? AND ?
            "#,
        )
        .bind(provider_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await?;

        use sqlx::Row;
        Ok((
            row.try_get("request_count")?,
            row.try_get("total_tokens")?,
            row.try_get("total_cost")?,
        ))
    }

    pub async fn get_error_rate(
        &self,
        provider_id: &str,
        start_time: i64,
        end_time: i64,
    ) -> Result<f64> {
        let row = sqlx::query(
            r#"
            SELECT 
                COUNT(*) as total_count,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as error_count
            FROM usage_logs
            WHERE provider_id = ? AND created_at BETWEEN ? AND ?
            "#,
        )
        .bind(provider_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await?;

        use sqlx::Row;
        let total: i64 = row.try_get("total_count")?;
        let errors: i64 = row.try_get("error_count")?;

        if total == 0 {
            Ok(0.0)
        } else {
            Ok(errors as f64 / total as f64)
        }
    }

    fn row_to_usage_log(&self, row: sqlx::sqlite::SqliteRow) -> Result<UsageLog> {
        use sqlx::Row;

        Ok(UsageLog {
            id: row.try_get("id")?,
            session_id: row.try_get("session_id")?,
            provider_id: row.try_get("provider_id")?,
            model: row.try_get("model")?,
            prompt_tokens: row.try_get("prompt_tokens")?,
            completion_tokens: row.try_get("completion_tokens")?,
            total_tokens: row.try_get("total_tokens")?,
            cost_usd: row.try_get("cost_usd")?,
            latency_ms: row.try_get("latency_ms")?,
            status: row.try_get("status")?,
            error_message: row.try_get("error_message")?,
            created_at: row.try_get("created_at")?,
        })
    }
}
