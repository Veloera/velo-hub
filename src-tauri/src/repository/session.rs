use crate::error::{AppError, Result};
use crate::models::Session;
use sqlx::SqlitePool;

pub struct SessionRepository {
    pool: SqlitePool,
}

impl SessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, session: &Session) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, backend_id, created_at, last_activity, expired)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(&session.backend_id)
        .bind(session.created_at)
        .bind(session.last_activity)
        .bind(session.expired)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Session> {
        let row = sqlx::query(
            r#"
            SELECT id, backend_id, created_at, last_activity, expired
            FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Session with id {} not found", id)))?;

        use sqlx::Row;
        Ok(Session {
            id: row.try_get("id")?,
            backend_id: row.try_get("backend_id")?,
            created_at: row.try_get("created_at")?,
            last_activity: row.try_get("last_activity")?,
            expired: row.try_get("expired")?,
        })
    }

    pub async fn get_active(&self) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            r#"
            SELECT id, backend_id, created_at, last_activity, expired
            FROM sessions
            WHERE expired = FALSE
            ORDER BY last_activity DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        use sqlx::Row;
        rows.into_iter()
            .map(|row| {
                Ok(Session {
                    id: row.try_get("id")?,
                    backend_id: row.try_get("backend_id")?,
                    created_at: row.try_get("created_at")?,
                    last_activity: row.try_get("last_activity")?,
                    expired: row.try_get("expired")?,
                })
            })
            .collect()
    }

    pub async fn update_activity(&self, id: &str, last_activity: i64) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET last_activity = ?
            WHERE id = ? AND expired = FALSE
            "#,
        )
        .bind(last_activity)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Active session with id {} not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn expire(&self, id: &str) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET expired = TRUE
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Session with id {} not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn expire_inactive(&self, threshold_timestamp: i64) -> Result<u64> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET expired = TRUE
            WHERE expired = FALSE AND last_activity < ?
            "#,
        )
        .bind(threshold_timestamp)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Session with id {} not found",
                id
            )));
        }

        Ok(())
    }

    pub async fn get_history(&self, limit: i64, offset: i64) -> Result<Vec<Session>> {
        let rows = sqlx::query(
            r#"
            SELECT id, backend_id, created_at, last_activity, expired
            FROM sessions
            ORDER BY created_at DESC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        use sqlx::Row;
        rows.into_iter()
            .map(|row| {
                Ok(Session {
                    id: row.try_get("id")?,
                    backend_id: row.try_get("backend_id")?,
                    created_at: row.try_get("created_at")?,
                    last_activity: row.try_get("last_activity")?,
                    expired: row.try_get("expired")?,
                })
            })
            .collect()
    }
}
