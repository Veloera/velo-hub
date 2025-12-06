use crate::error::{AppError, Result};
use crate::models::DecisionRecord;
use sqlx::SqlitePool;
use uuid::Uuid;

pub struct DecisionChainRepository {
    pub db: SqlitePool,
    pool: SqlitePool,
}

impl DecisionChainRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            db: pool.clone(),
            pool,
        }
    }

    /// Create a decision record in the database
    pub async fn create(&self, usage_log_id: &str, decision: &DecisionRecord) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO decision_chains (id, usage_log_id, step_number, action, provider_id, reason, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(usage_log_id)
        .bind(decision.step_number as i32)
        .bind(decision.action.to_string())
        .bind(&decision.provider_id)
        .bind(&decision.reason)
        .bind(decision.timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Create multiple decision records in a batch
    pub async fn create_batch(
        &self,
        usage_log_id: &str,
        decisions: &[DecisionRecord],
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for decision in decisions {
            sqlx::query(
                r#"
                INSERT INTO decision_chains (id, usage_log_id, step_number, action, provider_id, reason, timestamp)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(Uuid::new_v4().to_string())
            .bind(usage_log_id)
            .bind(decision.step_number as i32)
            .bind(decision.action.to_string())
            .bind(&decision.provider_id)
            .bind(&decision.reason)
            .bind(decision.timestamp)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get decision chain for a usage log
    pub async fn get_by_usage_log(&self, usage_log_id: &str) -> Result<Vec<DecisionRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT step_number, action, provider_id, reason, timestamp
            FROM decision_chains
            WHERE usage_log_id = ?
            ORDER BY step_number ASC
            "#,
        )
        .bind(usage_log_id)
        .fetch_all(&self.pool)
        .await?;

        use sqlx::Row;
        rows.into_iter()
            .map(|row| {
                let action_str: String = row.try_get("action")?;
                let action = match action_str.as_str() {
                    "selected" => crate::models::DecisionAction::Selected,
                    "failed" => crate::models::DecisionAction::Failed,
                    "retried" => crate::models::DecisionAction::Retried,
                    "redirected" => crate::models::DecisionAction::Redirected,
                    "circuit_breaker_open" => crate::models::DecisionAction::CircuitBreakerOpen,
                    "rate_limited" => crate::models::DecisionAction::RateLimited,
                    _ => {
                        return Err(AppError::Internal(format!(
                            "Invalid decision action: {}",
                            action_str
                        )))
                    }
                };

                Ok(DecisionRecord {
                    step_number: row.try_get::<i32, _>("step_number")? as u32,
                    action,
                    provider_id: row.try_get("provider_id")?,
                    reason: row.try_get("reason")?,
                    timestamp: row.try_get("timestamp")?,
                })
            })
            .collect()
    }

    /// Delete decision chains for a usage log
    pub async fn delete_by_usage_log(&self, usage_log_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM decision_chains WHERE usage_log_id = ?")
            .bind(usage_log_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
