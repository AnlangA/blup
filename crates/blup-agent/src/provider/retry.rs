use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use tokio::time::sleep;
use tracing;

use super::types::*;
use super::{LlmError, LlmProvider};

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay before first retry.
    pub initial_delay: Duration,
    /// Maximum delay between retries.
    pub max_delay: Duration,
    /// Backoff multiplier (exponential backoff).
    pub backoff_multiplier: f64,
    /// Whether to retry on rate limiting.
    pub retry_on_rate_limit: bool,
    /// Whether to retry on transient errors.
    pub retry_on_transient: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            retry_on_rate_limit: true,
            retry_on_transient: true,
        }
    }
}

/// A provider wrapper that adds retry logic with exponential backoff.
pub struct RetryProvider {
    inner: Arc<dyn LlmProvider>,
    config: RetryConfig,
}

impl RetryProvider {
    pub fn new(inner: Arc<dyn LlmProvider>, config: RetryConfig) -> Self {
        Self { inner, config }
    }

    pub fn with_default_config(inner: Arc<dyn LlmProvider>) -> Self {
        Self::new(inner, RetryConfig::default())
    }

    /// Calculate delay for a given attempt number.
    fn delay_for_attempt(&self, attempt: u32) -> Duration {
        let delay_ms = self.config.initial_delay.as_millis() as f64
            * self.config.backoff_multiplier.powi(attempt as i32);
        let delay = Duration::from_millis(delay_ms as u64);
        delay.min(self.config.max_delay)
    }

    /// Check if an error is retryable.
    fn is_retryable(&self, error: &LlmError) -> bool {
        match error {
            LlmError::RateLimited { .. } => self.config.retry_on_rate_limit,
            LlmError::Unavailable(_) => self.config.retry_on_transient,
            LlmError::Http(_) => self.config.retry_on_transient,
            LlmError::StreamEnded => true,
            _ => false,
        }
    }

    /// Execute a request with retry logic.
    async fn execute_with_retry<F, Fut, T>(&self, operation: F) -> Result<T, LlmError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, LlmError>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    if attempt < self.config.max_retries && self.is_retryable(&error) {
                        let delay = self.delay_for_attempt(attempt);
                        tracing::warn!(
                            attempt = attempt + 1,
                            max_retries = self.config.max_retries,
                            delay_ms = delay.as_millis(),
                            error = %error,
                            "Retrying after transient error"
                        );
                        sleep(delay).await;
                        last_error = Some(error);
                    } else {
                        return Err(error);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::Unavailable("Max retries exceeded".to_string())))
    }
}

#[async_trait]
impl LlmProvider for RetryProvider {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn model(&self) -> &str {
        self.inner.model()
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let inner = Arc::clone(&self.inner);
        self.execute_with_retry(|| {
            let req = request.clone();
            let provider = Arc::clone(&inner);
            async move { provider.complete(req).await }
        })
        .await
    }

    fn stream(
        &self,
        request: LlmRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>> {
        // For streaming, we delegate directly to the inner provider
        // since retrying mid-stream is complex and often not useful.
        self.inner.stream(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::mock::MockProvider;

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let mock = Arc::new(MockProvider::new());
        mock.push_response("Success");

        let retry = RetryProvider::with_default_config(mock.clone());
        let request = LlmRequest {
            model: "test".to_string(),
            messages: vec![LlmMessage::user("test")],
            temperature: None,
            max_tokens: None,
            stream: false,
        };

        let result = retry.complete(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_config_delay_calculation() {
        let config = RetryConfig {
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(10),
            ..Default::default()
        };

        let mock = Arc::new(MockProvider::new());
        let retry = RetryProvider::new(mock, config);

        assert_eq!(retry.delay_for_attempt(0), Duration::from_millis(100));
        assert_eq!(retry.delay_for_attempt(1), Duration::from_millis(200));
        assert_eq!(retry.delay_for_attempt(2), Duration::from_millis(400));
    }
}
