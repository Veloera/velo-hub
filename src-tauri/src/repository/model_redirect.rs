use crate::error::AppError;
use crate::models::ModelRedirect;
use sqlx::SqlitePool;

pub struct ModelRedirectRepository {
    pub db: SqlitePool,
}

impl ModelRedirectRepository {
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Get all model redirects
    pub async fn get_all(&self) -> Result<Vec<ModelRedirect>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT id, source_model, target_model, target_provider_id, enabled, created_at
            FROM model_redirects
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        use sqlx::Row;
        let redirects = rows
            .into_iter()
            .map(|row| ModelRedirect {
                id: row.get("id"),
                source_model: row.get("source_model"),
                target_model: row.get("target_model"),
                target_provider_id: row.get("target_provider_id"),
                enabled: row.get("enabled"),
                created_at: row.get("created_at"),
            })
            .collect();

        Ok(redirects)
    }

    /// Get a model redirect by ID
    pub async fn get_by_id(&self, id: &str) -> Result<ModelRedirect, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, source_model, target_model, target_provider_id, enabled, created_at
            FROM model_redirects
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&self.db)
        .await?;

        use sqlx::Row;
        Ok(ModelRedirect {
            id: row.get("id"),
            source_model: row.get("source_model"),
            target_model: row.get("target_model"),
            target_provider_id: row.get("target_provider_id"),
            enabled: row.get("enabled"),
            created_at: row.get("created_at"),
        })
    }

    /// Get a model redirect by source model name
    pub async fn get_by_source_model(
        &self,
        source_model: &str,
    ) -> Result<Option<ModelRedirect>, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, source_model, target_model, target_provider_id, enabled, created_at
            FROM model_redirects
            WHERE source_model = ? AND enabled = TRUE
            "#,
        )
        .bind(source_model)
        .fetch_optional(&self.db)
        .await?;

        use sqlx::Row;
        Ok(row.map(|row| ModelRedirect {
            id: row.get("id"),
            source_model: row.get("source_model"),
            target_model: row.get("target_model"),
            target_provider_id: row.get("target_provider_id"),
            enabled: row.get("enabled"),
            created_at: row.get("created_at"),
        }))
    }

    /// Create a new model redirect
    pub async fn create(&self, redirect: &ModelRedirect) -> Result<(), AppError> {
        sqlx::query(
            r#"
            INSERT INTO model_redirects (id, source_model, target_model, target_provider_id, enabled, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&redirect.id)
        .bind(&redirect.source_model)
        .bind(&redirect.target_model)
        .bind(&redirect.target_provider_id)
        .bind(redirect.enabled)
        .bind(redirect.created_at)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Update an existing model redirect
    pub async fn update(&self, redirect: &ModelRedirect) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE model_redirects
            SET source_model = ?, target_model = ?, target_provider_id = ?, enabled = ?
            WHERE id = ?
            "#,
        )
        .bind(&redirect.source_model)
        .bind(&redirect.target_model)
        .bind(&redirect.target_provider_id)
        .bind(redirect.enabled)
        .bind(&redirect.id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Delete a model redirect
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        sqlx::query(
            r#"
            DELETE FROM model_redirects WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Toggle enabled status
    pub async fn toggle_enabled(&self, id: &str, enabled: bool) -> Result<(), AppError> {
        sqlx::query(
            r#"
            UPDATE model_redirects
            SET enabled = ?
            WHERE id = ?
            "#,
        )
        .bind(enabled)
        .bind(id)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Check if a source model already has a redirect rule
    pub async fn exists_for_source(
        &self,
        source_model: &str,
        exclude_id: Option<&str>,
    ) -> Result<bool, AppError> {
        let count: i64 = if let Some(id) = exclude_id {
            let row = sqlx::query(
                r#"
                SELECT COUNT(*) as count FROM model_redirects
                WHERE source_model = ? AND id != ?
                "#,
            )
            .bind(source_model)
            .bind(id)
            .fetch_one(&self.db)
            .await?;

            use sqlx::Row;
            row.get("count")
        } else {
            let row = sqlx::query(
                r#"
                SELECT COUNT(*) as count FROM model_redirects
                WHERE source_model = ?
                "#,
            )
            .bind(source_model)
            .fetch_one(&self.db)
            .await?;

            use sqlx::Row;
            row.get("count")
        };

        Ok(count > 0)
    }
}
