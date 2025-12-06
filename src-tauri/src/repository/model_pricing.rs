use crate::error::{AppError, Result};
use crate::models::ModelPricing;
use sqlx::SqlitePool;

pub struct ModelPricingRepository {
    pool: SqlitePool,
}

impl ModelPricingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, pricing: &ModelPricing) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO model_pricing (
                id, model_name, provider, input_cost_per_1m, output_cost_per_1m, last_updated
            ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&pricing.id)
        .bind(&pricing.model_name)
        .bind(&pricing.provider)
        .bind(pricing.input_cost_per_1m)
        .bind(pricing.output_cost_per_1m)
        .bind(pricing.last_updated)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_model_name(&self, model_name: &str) -> Result<ModelPricing> {
        let row = sqlx::query(
            r#"
            SELECT id, model_name, provider, input_cost_per_1m, output_cost_per_1m, last_updated
            FROM model_pricing
            WHERE model_name = ?
            "#,
        )
        .bind(model_name)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Pricing for model {} not found", model_name)))?;

        self.row_to_model_pricing(row)
    }

    pub async fn search(&self, query: &str) -> Result<Vec<ModelPricing>> {
        let search_pattern = format!("%{}%", query);
        let rows = sqlx::query(
            r#"
            SELECT id, model_name, provider, input_cost_per_1m, output_cost_per_1m, last_updated
            FROM model_pricing
            WHERE model_name LIKE ? OR provider LIKE ?
            ORDER BY model_name ASC
            LIMIT 100
            "#,
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_model_pricing(row))
            .collect()
    }

    pub async fn get_paginated(&self, limit: i64, offset: i64) -> Result<Vec<ModelPricing>> {
        let rows = sqlx::query(
            r#"
            SELECT id, model_name, provider, input_cost_per_1m, output_cost_per_1m, last_updated
            FROM model_pricing
            ORDER BY model_name ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_model_pricing(row))
            .collect()
    }

    pub async fn get_all(&self, limit: i64, offset: i64) -> Result<Vec<ModelPricing>> {
        let rows = sqlx::query(
            r#"
            SELECT id, model_name, provider, input_cost_per_1m, output_cost_per_1m, last_updated
            FROM model_pricing
            ORDER BY model_name ASC
            LIMIT ? OFFSET ?
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_model_pricing(row))
            .collect()
    }

    pub async fn update(&self, pricing: &ModelPricing) -> Result<()> {
        let result = sqlx::query(
            r#"
            UPDATE model_pricing
            SET provider = ?, input_cost_per_1m = ?, output_cost_per_1m = ?, last_updated = ?
            WHERE model_name = ?
            "#,
        )
        .bind(&pricing.provider)
        .bind(pricing.input_cost_per_1m)
        .bind(pricing.output_cost_per_1m)
        .bind(pricing.last_updated)
        .bind(&pricing.model_name)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Pricing for model {} not found",
                pricing.model_name
            )));
        }

        Ok(())
    }

    pub async fn upsert(&self, pricing: &ModelPricing) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO model_pricing (
                id, model_name, provider, input_cost_per_1m, output_cost_per_1m, last_updated
            ) VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(model_name) DO UPDATE SET
                provider = excluded.provider,
                input_cost_per_1m = excluded.input_cost_per_1m,
                output_cost_per_1m = excluded.output_cost_per_1m,
                last_updated = excluded.last_updated
            "#,
        )
        .bind(&pricing.id)
        .bind(&pricing.model_name)
        .bind(&pricing.provider)
        .bind(pricing.input_cost_per_1m)
        .bind(pricing.output_cost_per_1m)
        .bind(pricing.last_updated)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete(&self, model_name: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM model_pricing WHERE model_name = ?")
            .bind(model_name)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Pricing for model {} not found",
                model_name
            )));
        }

        Ok(())
    }

    pub async fn count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM model_pricing")
            .fetch_one(&self.pool)
            .await?;

        use sqlx::Row;
        Ok(row.try_get("count")?)
    }

    fn row_to_model_pricing(&self, row: sqlx::sqlite::SqliteRow) -> Result<ModelPricing> {
        use sqlx::Row;

        Ok(ModelPricing {
            id: row.try_get("id")?,
            model_name: row.try_get("model_name")?,
            provider: row.try_get("provider")?,
            input_cost_per_1m: row.try_get("input_cost_per_1m")?,
            output_cost_per_1m: row.try_get("output_cost_per_1m")?,
            last_updated: row.try_get("last_updated")?,
        })
    }
}
