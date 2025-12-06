use crate::error::{ProviderError, Result};
use crate::models::{ChatCompletionRequest, ChatCompletionResponse, Provider};
use crate::provider::ProviderAdapter;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

pub struct CustomAdapter {
    client: Client,
}

impl CustomAdapter {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    fn build_client_with_proxy(&self, provider: &Provider) -> Result<Client> {
        let mut builder = Client::builder()
            .timeout(Duration::from_secs(120))
            .pool_max_idle_per_host(10);

        if let Some(proxy_config) = &provider.proxy {
            let proxy = reqwest::Proxy::all(&proxy_config.url)
                .map_err(|e| ProviderError::ProxyError(e.to_string()))?;

            if let Some(auth) = &proxy_config.auth {
                let proxy = proxy.basic_auth(&auth.username, &auth.password);
                builder = builder.proxy(proxy);
            } else {
                builder = builder.proxy(proxy);
            }
        }

        builder
            .build()
            .map_err(|e| ProviderError::ConnectionFailed(e.to_string()).into())
    }
}

#[async_trait]
impl ProviderAdapter for CustomAdapter {
    async fn send_request(
        &self,
        request: &ChatCompletionRequest,
        config: &Provider,
    ) -> Result<ChatCompletionResponse> {
        let client = if config.proxy.is_some() {
            self.build_client_with_proxy(config)?
        } else {
            self.client.clone()
        };

        let mut req_builder = client
            .post(&format!(
                "{}/chat/completions",
                config.endpoint.trim_end_matches('/')
            ))
            .header("Content-Type", "application/json")
            .json(request);

        if !config.api_key.is_empty() {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", config.api_key));
        }

        let response = req_builder.send().await.map_err(|e| {
            if e.is_timeout() {
                ProviderError::Timeout(format!("Request timeout: {}", e))
            } else if e.is_connect() {
                ProviderError::ConnectionFailed(format!("Connection failed: {}", e))
            } else {
                ProviderError::ApiError(format!("Request failed: {}", e))
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            return Err(match status.as_u16() {
                401 | 403 => ProviderError::AuthenticationFailed(error_text),
                429 => ProviderError::ApiError(format!("Rate limited: {}", error_text)),
                500..=599 => ProviderError::ApiError(format!("Server error: {}", error_text)),
                _ => ProviderError::ApiError(format!("API error ({}): {}", status, error_text)),
            }
            .into());
        }

        let completion: ChatCompletionResponse = response.json().await.map_err(|e| {
            ProviderError::ResponseError(format!("Failed to parse response: {}", e))
        })?;

        Ok(completion)
    }

    async fn validate_config(&self, config: &Provider) -> Result<()> {
        if config.endpoint.is_empty() {
            return Err(
                ProviderError::InvalidConfig("Endpoint cannot be empty".to_string()).into(),
            );
        }

        if !config.endpoint.starts_with("http://") && !config.endpoint.starts_with("https://") {
            return Err(ProviderError::InvalidConfig(
                "Endpoint must start with http:// or https://".to_string(),
            )
            .into());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ProviderType;

    #[tokio::test]
    async fn test_validate_config_success() {
        let adapter = CustomAdapter::new();
        let provider = Provider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::Custom,
            endpoint: "https://custom-api.example.com/v1".to_string(),
            api_key: "".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(adapter.validate_config(&provider).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_with_api_key() {
        let adapter = CustomAdapter::new();
        let provider = Provider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::Custom,
            endpoint: "https://custom-api.example.com/v1".to_string(),
            api_key: "custom-key-123".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(adapter.validate_config(&provider).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_config_empty_endpoint() {
        let adapter = CustomAdapter::new();
        let provider = Provider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::Custom,
            endpoint: "".to_string(),
            api_key: "".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(adapter.validate_config(&provider).await.is_err());
    }
}
