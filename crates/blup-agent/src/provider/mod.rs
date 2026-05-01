pub mod anthropic;
pub mod mock;
pub mod ollama;
pub mod openai;
pub mod retry;
pub mod transform;
pub mod types;

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use thiserror::Error;

pub use retry::{RetryConfig, RetryProvider};
pub use types::{LlmMessage, LlmRequest, LlmResponse, Role, StreamChunk, TokenUsage};

use crate::config::{ProviderConfig, ProviderType};

/// Errors from LLM providers.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Provider error {status}: {body}")]
    ProviderError { status: u16, body: String },

    #[error("Stream ended unexpectedly")]
    StreamEnded,

    #[error("Response validation failed: {0}")]
    Validation(String),

    #[error("Provider unavailable: {0}")]
    Unavailable(String),

    #[error("JSON parse error: {0}")]
    JsonParse(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Authentication failed: {0}")]
    Auth(String),
}

/// Unified LLM provider trait.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Provider name for logging.
    fn name(&self) -> &str;

    /// Model identifier.
    fn model(&self) -> &str;

    /// Non-streaming completion.
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Streaming completion.
    fn stream(
        &self,
        request: LlmRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>;
}

/// Factory for creating providers from configuration.
pub struct ProviderFactory;

impl ProviderFactory {
    pub fn from_config(config: &ProviderConfig) -> Result<Arc<dyn LlmProvider>, LlmError> {
        let provider: Arc<dyn LlmProvider> = match config.provider_type {
            ProviderType::OpenAI => {
                let provider = openai::OpenAiProvider::new(config)?;
                Arc::new(provider)
            }
            ProviderType::Anthropic => {
                let provider = anthropic::AnthropicProvider::new(config)?;
                Arc::new(provider)
            }
            ProviderType::Ollama => {
                let provider = ollama::OllamaProvider::new(config)?;
                Arc::new(provider)
            }
            ProviderType::Mock => Arc::new(mock::MockProvider::new()),
        };

        // Wrap with retry logic for non-mock providers
        if config.provider_type != ProviderType::Mock && config.max_retries > 0 {
            let retry_config = RetryConfig {
                max_retries: config.max_retries,
                ..Default::default()
            };
            Ok(Arc::new(RetryProvider::new(provider, retry_config)))
        } else {
            Ok(provider)
        }
    }
}
