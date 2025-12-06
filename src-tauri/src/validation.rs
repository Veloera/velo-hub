use crate::error::{AppError, Result};
use crate::models::{Provider, ProviderType, ProxyConfig};
use validator::{Validate, ValidationError};

/// Validation schemas for API inputs

#[derive(Debug, Validate)]
pub struct CreateProviderRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: String,

    pub provider_type: ProviderType,

    #[validate(url(message = "Invalid endpoint URL"))]
    pub endpoint: String,

    #[validate(length(min = 10, message = "API key must be at least 10 characters"))]
    pub api_key: String,

    #[validate(range(min = -100, max = 100, message = "Priority must be between -100 and 100"))]
    pub priority: i32,

    #[validate(range(min = 1, max = 1000, message = "Weight must be between 1 and 1000"))]
    pub weight: i32,

    pub enabled: bool,

    #[validate(nested)]
    pub proxy: Option<ProxyConfig>,
}

#[derive(Debug, Validate)]
pub struct UpdateProviderRequest {
    #[validate(length(
        min = 1,
        max = 100,
        message = "Name must be between 1 and 100 characters"
    ))]
    pub name: Option<String>,

    pub provider_type: Option<ProviderType>,

    #[validate(url(message = "Invalid endpoint URL"))]
    pub endpoint: Option<String>,

    #[validate(length(min = 10, message = "API key must be at least 10 characters"))]
    pub api_key: Option<String>,

    #[validate(range(min = -100, max = 100, message = "Priority must be between -100 and 100"))]
    pub priority: Option<i32>,

    #[validate(range(min = 1, max = 1000, message = "Weight must be between 1 and 1000"))]
    pub weight: Option<i32>,

    pub enabled: Option<bool>,

    #[validate(nested)]
    pub proxy: Option<ProxyConfig>,
}

impl Validate for ProxyConfig {
    fn validate(&self) -> std::result::Result<(), validator::ValidationErrors> {
        let mut errors = validator::ValidationErrors::new();

        // Validate proxy URL format
        if !is_valid_url(&self.url) {
            errors.add("url", ValidationError::new("Invalid proxy URL format"));
        }

        // Validate auth if present
        if let Some(ref auth) = self.auth {
            if auth.username.is_empty() {
                errors.add(
                    "username",
                    ValidationError::new("Proxy username cannot be empty"),
                );
            }
            if auth.password.is_empty() {
                errors.add(
                    "password",
                    ValidationError::new("Proxy password cannot be empty"),
                );
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Helper function to validate URL format
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("socks5://")
}

/// Validate a Provider configuration
pub fn validate_provider(provider: &Provider) -> Result<()> {
    // Validate name
    if provider.name.is_empty() || provider.name.len() > 100 {
        return Err(AppError::InvalidInput(
            "Provider name must be between 1 and 100 characters".to_string(),
        ));
    }

    // Validate endpoint URL
    if !is_valid_url(&provider.endpoint) {
        return Err(AppError::InvalidInput(
            "Invalid endpoint URL format".to_string(),
        ));
    }

    // Validate API key
    if provider.api_key.len() < 10 {
        return Err(AppError::InvalidInput(
            "API key must be at least 10 characters".to_string(),
        ));
    }

    // Validate priority
    if provider.priority < -100 || provider.priority > 100 {
        return Err(AppError::InvalidInput(
            "Priority must be between -100 and 100".to_string(),
        ));
    }

    // Validate weight
    if provider.weight < 1 || provider.weight > 1000 {
        return Err(AppError::InvalidInput(
            "Weight must be between 1 and 1000".to_string(),
        ));
    }

    // Validate proxy if present
    if let Some(ref proxy) = provider.proxy {
        proxy
            .validate()
            .map_err(|e| AppError::InvalidInput(format!("Invalid proxy configuration: {}", e)))?;
    }

    Ok(())
}

/// Validate endpoint connectivity (basic URL format check)
pub fn validate_endpoint_format(endpoint: &str) -> Result<()> {
    // Check if it's HTTP or HTTPS
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        return Err(AppError::InvalidInput(
            "Endpoint must use HTTP or HTTPS protocol".to_string(),
        ));
    }

    Ok(())
}

/// Validate model name format
pub fn validate_model_name(model: &str) -> Result<()> {
    if model.is_empty() {
        return Err(AppError::InvalidInput(
            "Model name cannot be empty".to_string(),
        ));
    }

    if model.len() > 200 {
        return Err(AppError::InvalidInput(
            "Model name must not exceed 200 characters".to_string(),
        ));
    }

    Ok(())
}

/// Validate rate limit configuration
pub fn validate_rate_limit_config(
    rpm: Option<u32>,
    tokens_per_5h: Option<u64>,
    tokens_per_week: Option<u64>,
    tokens_per_month: Option<u64>,
    max_concurrent_sessions: Option<u32>,
) -> Result<()> {
    if let Some(rpm) = rpm {
        if rpm == 0 {
            return Err(AppError::InvalidInput(
                "RPM must be greater than 0".to_string(),
            ));
        }
        if rpm > 1_000_000 {
            return Err(AppError::InvalidInput(
                "RPM must not exceed 1,000,000".to_string(),
            ));
        }
    }

    if let Some(tokens) = tokens_per_5h {
        if tokens == 0 {
            return Err(AppError::InvalidInput(
                "Token quota must be greater than 0".to_string(),
            ));
        }
    }

    if let Some(tokens) = tokens_per_week {
        if tokens == 0 {
            return Err(AppError::InvalidInput(
                "Token quota must be greater than 0".to_string(),
            ));
        }
    }

    if let Some(tokens) = tokens_per_month {
        if tokens == 0 {
            return Err(AppError::InvalidInput(
                "Token quota must be greater than 0".to_string(),
            ));
        }
    }

    if let Some(sessions) = max_concurrent_sessions {
        if sessions == 0 {
            return Err(AppError::InvalidInput(
                "Max concurrent sessions must be greater than 0".to_string(),
            ));
        }
        if sessions > 10_000 {
            return Err(AppError::InvalidInput(
                "Max concurrent sessions must not exceed 10,000".to_string(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ProxyAuth, ProxyType};

    #[test]
    fn test_validate_provider_valid() {
        let provider = Provider {
            id: "test-id".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::OpenAI,
            endpoint: "https://api.openai.com/v1".to_string(),
            api_key: "sk-1234567890".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(validate_provider(&provider).is_ok());
    }

    #[test]
    fn test_validate_provider_invalid_name() {
        let provider = Provider {
            id: "test-id".to_string(),
            name: "".to_string(),
            provider_type: ProviderType::OpenAI,
            endpoint: "https://api.openai.com/v1".to_string(),
            api_key: "sk-1234567890".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(validate_provider(&provider).is_err());
    }

    #[test]
    fn test_validate_provider_invalid_endpoint() {
        let provider = Provider {
            id: "test-id".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::OpenAI,
            endpoint: "not-a-url".to_string(),
            api_key: "sk-1234567890".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(validate_provider(&provider).is_err());
    }

    #[test]
    fn test_validate_provider_invalid_api_key() {
        let provider = Provider {
            id: "test-id".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::OpenAI,
            endpoint: "https://api.openai.com/v1".to_string(),
            api_key: "short".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: None,
            created_at: 0,
            updated_at: 0,
        };

        assert!(validate_provider(&provider).is_err());
    }

    #[test]
    fn test_validate_provider_with_proxy() {
        let provider = Provider {
            id: "test-id".to_string(),
            name: "Test Provider".to_string(),
            provider_type: ProviderType::OpenAI,
            endpoint: "https://api.openai.com/v1".to_string(),
            api_key: "sk-1234567890".to_string(),
            priority: 0,
            weight: 1,
            enabled: true,
            proxy: Some(ProxyConfig {
                proxy_type: ProxyType::Http,
                url: "http://proxy.example.com:8080".to_string(),
                auth: Some(ProxyAuth {
                    username: "user".to_string(),
                    password: "pass".to_string(),
                }),
            }),
            created_at: 0,
            updated_at: 0,
        };

        assert!(validate_provider(&provider).is_ok());
    }

    #[test]
    fn test_validate_endpoint_format() {
        assert!(validate_endpoint_format("https://api.example.com").is_ok());
        assert!(validate_endpoint_format("http://localhost:8080").is_ok());
        assert!(validate_endpoint_format("ftp://example.com").is_err());
        assert!(validate_endpoint_format("not-a-url").is_err());
    }

    #[test]
    fn test_validate_model_name() {
        assert!(validate_model_name("gpt-4").is_ok());
        assert!(validate_model_name("claude-3-opus").is_ok());
        assert!(validate_model_name("").is_err());
        assert!(validate_model_name(&"a".repeat(201)).is_err());
    }

    #[test]
    fn test_validate_rate_limit_config() {
        assert!(validate_rate_limit_config(Some(100), None, None, None, None).is_ok());
        assert!(validate_rate_limit_config(Some(0), None, None, None, None).is_err());
        assert!(validate_rate_limit_config(None, None, None, None, Some(0)).is_err());
        assert!(validate_rate_limit_config(None, None, None, None, Some(100)).is_ok());
    }
}
