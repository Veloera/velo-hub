use crate::error::{ProviderError, Result};
use crate::models::ProviderType;
use crate::provider::adapter::ProviderAdapter;
use crate::provider::anthropic::AnthropicAdapter;
use crate::provider::custom::CustomAdapter;
use crate::provider::gemini::GeminiAdapter;
use crate::provider::openai::OpenAIAdapter;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ProviderRegistry {
    adapters: HashMap<ProviderType, Arc<dyn ProviderAdapter>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut adapters: HashMap<ProviderType, Arc<dyn ProviderAdapter>> = HashMap::new();

        adapters.insert(
            ProviderType::OpenAI,
            Arc::new(OpenAIAdapter::new()) as Arc<dyn ProviderAdapter>,
        );
        adapters.insert(
            ProviderType::Anthropic,
            Arc::new(AnthropicAdapter::new()) as Arc<dyn ProviderAdapter>,
        );
        adapters.insert(
            ProviderType::Gemini,
            Arc::new(GeminiAdapter::new()) as Arc<dyn ProviderAdapter>,
        );
        adapters.insert(
            ProviderType::Custom,
            Arc::new(CustomAdapter::new()) as Arc<dyn ProviderAdapter>,
        );

        Self { adapters }
    }

    pub fn get_adapter(&self, provider_type: &ProviderType) -> Result<Arc<dyn ProviderAdapter>> {
        self.adapters.get(provider_type).cloned().ok_or_else(|| {
            ProviderError::UnsupportedType(format!(
                "Provider type '{}' is not supported",
                provider_type
            ))
            .into()
        })
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ProviderRegistry::new();
        assert!(registry.get_adapter(&ProviderType::OpenAI).is_ok());
        assert!(registry.get_adapter(&ProviderType::Anthropic).is_ok());
        assert!(registry.get_adapter(&ProviderType::Gemini).is_ok());
        assert!(registry.get_adapter(&ProviderType::Custom).is_ok());
    }

    #[test]
    fn test_get_openai_adapter() {
        let registry = ProviderRegistry::new();
        let adapter = registry.get_adapter(&ProviderType::OpenAI);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_get_anthropic_adapter() {
        let registry = ProviderRegistry::new();
        let adapter = registry.get_adapter(&ProviderType::Anthropic);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_get_gemini_adapter() {
        let registry = ProviderRegistry::new();
        let adapter = registry.get_adapter(&ProviderType::Gemini);
        assert!(adapter.is_ok());
    }

    #[test]
    fn test_get_custom_adapter() {
        let registry = ProviderRegistry::new();
        let adapter = registry.get_adapter(&ProviderType::Custom);
        assert!(adapter.is_ok());
    }
}
