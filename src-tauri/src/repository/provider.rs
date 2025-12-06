use crate::error::{AppError, Result};
use crate::models::{Provider, ProviderType, ProxyAuth, ProxyConfig, ProxyType};
use sqlx::SqlitePool;

pub struct ProviderRepository {
    pool: SqlitePool,
}

impl ProviderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, provider: &Provider) -> Result<()> {
        let provider_type = serde_json::to_string(&provider.provider_type)?;
        let proxy_type = provider
            .proxy
            .as_ref()
            .map(|p| serde_json::to_string(&p.proxy_type))
            .transpose()?;
        let proxy_url = provider.proxy.as_ref().map(|p| p.url.clone());
        let proxy_auth = provider
            .proxy
            .as_ref()
            .and_then(|p| p.auth.as_ref())
            .map(|auth| serde_json::to_vec(auth))
            .transpose()?;

        sqlx::query(
            r#"
            INSERT INTO providers (
                id, name, provider_type, endpoint, api_key_encrypted,
                priority, weight, enabled, proxy_type, proxy_url, proxy_auth_encrypted,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(provider_type)
        .bind(&provider.endpoint)
        .bind(provider.api_key.as_bytes())
        .bind(provider.priority)
        .bind(provider.weight)
        .bind(provider.enabled)
        .bind(proxy_type)
        .bind(proxy_url)
        .bind(proxy_auth)
        .bind(provider.created_at)
        .bind(provider.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_by_id(&self, id: &str) -> Result<Provider> {
        let row = sqlx::query(
            r#"
            SELECT id, name, provider_type, endpoint, api_key_encrypted,
                   priority, weight, enabled, proxy_type, proxy_url, proxy_auth_encrypted,
                   created_at, updated_at
            FROM providers
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Provider with id {} not found", id)))?;

        self.row_to_provider(row)
    }

    pub async fn get_all(&self) -> Result<Vec<Provider>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, provider_type, endpoint, api_key_encrypted,
                   priority, weight, enabled, proxy_type, proxy_url, proxy_auth_encrypted,
                   created_at, updated_at
            FROM providers
            ORDER BY priority DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_provider(row))
            .collect()
    }

    pub async fn get_enabled(&self) -> Result<Vec<Provider>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, provider_type, endpoint, api_key_encrypted,
                   priority, weight, enabled, proxy_type, proxy_url, proxy_auth_encrypted,
                   created_at, updated_at
            FROM providers
            WHERE enabled = TRUE
            ORDER BY priority DESC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_provider(row))
            .collect()
    }

    pub async fn update(&self, provider: &Provider) -> Result<()> {
        let provider_type = serde_json::to_string(&provider.provider_type)?;
        let proxy_type = provider
            .proxy
            .as_ref()
            .map(|p| serde_json::to_string(&p.proxy_type))
            .transpose()?;
        let proxy_url = provider.proxy.as_ref().map(|p| p.url.clone());
        let proxy_auth = provider
            .proxy
            .as_ref()
            .and_then(|p| p.auth.as_ref())
            .map(|auth| serde_json::to_vec(auth))
            .transpose()?;

        let result = sqlx::query(
            r#"
            UPDATE providers
            SET name = ?, provider_type = ?, endpoint = ?, api_key_encrypted = ?,
                priority = ?, weight = ?, enabled = ?, proxy_type = ?, proxy_url = ?,
                proxy_auth_encrypted = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&provider.name)
        .bind(provider_type)
        .bind(&provider.endpoint)
        .bind(provider.api_key.as_bytes())
        .bind(provider.priority)
        .bind(provider.weight)
        .bind(provider.enabled)
        .bind(proxy_type)
        .bind(proxy_url)
        .bind(proxy_auth)
        .bind(provider.updated_at)
        .bind(&provider.id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Provider with id {} not found",
                provider.id
            )));
        }

        Ok(())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Provider with id {} not found",
                id
            )));
        }

        Ok(())
    }

    fn row_to_provider(&self, row: sqlx::sqlite::SqliteRow) -> Result<Provider> {
        use sqlx::Row;

        let id: String = row.try_get("id")?;
        let name: String = row.try_get("name")?;
        let provider_type_str: String = row.try_get("provider_type")?;
        let provider_type: ProviderType = serde_json::from_str(&provider_type_str)?;
        let endpoint: String = row.try_get("endpoint")?;
        let api_key_encrypted: Vec<u8> = row.try_get("api_key_encrypted")?;
        let api_key = String::from_utf8(api_key_encrypted)
            .map_err(|e| AppError::Database(sqlx::Error::Decode(Box::new(e))))?;
        let priority: i32 = row.try_get("priority")?;
        let weight: i32 = row.try_get("weight")?;
        let enabled: bool = row.try_get("enabled")?;
        let created_at: i64 = row.try_get("created_at")?;
        let updated_at: i64 = row.try_get("updated_at")?;

        let proxy = if let Ok(Some(proxy_type_str)) = row.try_get::<Option<String>, _>("proxy_type")
        {
            let proxy_type: ProxyType = serde_json::from_str(&proxy_type_str)?;
            let proxy_url: String = row.try_get("proxy_url")?;
            let proxy_auth = if let Ok(Some(auth_bytes)) =
                row.try_get::<Option<Vec<u8>>, _>("proxy_auth_encrypted")
            {
                Some(serde_json::from_slice::<ProxyAuth>(&auth_bytes)?)
            } else {
                None
            };

            Some(ProxyConfig {
                proxy_type,
                url: proxy_url,
                auth: proxy_auth,
            })
        } else {
            None
        };

        Ok(Provider {
            id,
            name,
            provider_type,
            endpoint,
            api_key,
            priority,
            weight,
            enabled,
            proxy,
            created_at,
            updated_at,
        })
    }
}
