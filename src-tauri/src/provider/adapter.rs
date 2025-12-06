// Provider adapter trait - to be implemented in task 5.1
use crate::error::Result;
use crate::models::{ChatCompletionRequest, ChatCompletionResponse, Provider};
use async_trait::async_trait;

#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    async fn send_request(
        &self,
        request: &ChatCompletionRequest,
        config: &Provider,
    ) -> Result<ChatCompletionResponse>;

    async fn validate_config(&self, config: &Provider) -> Result<()>;
}
