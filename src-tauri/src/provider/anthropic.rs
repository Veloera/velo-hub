use crate::converter::{AnthropicResponse, FormatConverter};
use crate::error::{ProviderError, Result};
use crate::models::{ChatCompletionRequest, ChatCompletionResponse, Provider};
use crate::provider::ProviderAdapter;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

pub struct AnthropicAdapter {
    client: Client,
}

impl AnthropicAdapter {
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
impl ProviderAdapter for AnthropicAdapter {
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

        let anthropic_request = FormatConverter::openai_to_anthropic(request)?;

        let response = client
            .post(&format!(
                "{}/messages",
                config.endpoint.trim_end_matches('/')
            ))
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .map_err(|e| {
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

        let anthropic_response: AnthropicResponse = response.json().await.map_err(|e| {
            ProviderError::ResponseError(format!("Failed to parse response: {}", e))
        })?;

        FormatConverter::anthropic_to_openai(anthropic_response, &request.model)
    }

    async fn validate_config(&self, config: &Provider) -> Result<()> {
        if config.endpoint.is_empty() {
            return Err(
                ProviderError::InvalidConfig("Endpoint cannot be empty".to_string()).into(),
            );
        }

        if config.api_key.is_empty() {
            return Err(ProviderError::InvalidConfig("API key cannot be empty".to_string()).into());
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
        let adapter = AnthropicAdapter::new();
        let provider = Provider {
            id: "test".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::Anthropic,
            endpoint: "https://api.anthropic.com/v1".to_string(),
            api_key: "sk-ant-test123".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(adapter.validate_config(&provider).await.is_ok());
    }
}
