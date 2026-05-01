use std::pin::Pin;
use std::sync::Arc;

use async_openai::config::{Config, OpenAIConfig};
use async_openai::Client;
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use serde_json::{json, Value};

use super::types::*;
use super::{LlmError, LlmProvider};
use crate::config::ProviderConfig;

/// OpenAI-compatible provider using async-openai BYOT feature.
pub struct OpenAiProvider {
    client: Arc<Client<Box<dyn Config>>>,
    model: String,
    name: String,
}

impl OpenAiProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, LlmError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| LlmError::Config("Missing API key for OpenAI provider".to_string()))?;

        let mut openai_config = OpenAIConfig::new().with_api_key(api_key);

        if let Some(ref base_url) = config.base_url {
            openai_config = openai_config.with_api_base(base_url);
        }

        let client = Arc::new(Client::with_config(
            Box::new(openai_config) as Box<dyn Config>
        ));

        let name = if config.base_url.is_some() {
            format!(
                "openai-compatible({})",
                config.base_url.as_deref().unwrap_or("")
            )
        } else {
            "openai".to_string()
        };

        Ok(Self {
            client,
            model: config.model.clone(),
            name,
        })
    }

    fn convert_messages(messages: &[LlmMessage]) -> Vec<Value> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                json!({
                    "role": role,
                    "content": m.content
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let messages = Self::convert_messages(&request.messages);

        let body = json!({
            "model": self.model,
            "messages": messages,
            "temperature": request.temperature,
            "max_tokens": request.max_tokens,
            "stream": false
        });

        let response: Value =
            self.client
                .chat()
                .create_byot(body)
                .await
                .map_err(|e| LlmError::ProviderError {
                    status: 500,
                    body: e.to_string(),
                })?;

        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = TokenUsage {
            prompt_tokens: response["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: response["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: response["usage"]["total_tokens"].as_u64().unwrap_or(0) as u32,
        };

        Ok(LlmResponse {
            content,
            model: response["model"]
                .as_str()
                .unwrap_or(&self.model)
                .to_string(),
            provider: self.name.clone(),
            usage,
            finish_reason: response["choices"][0]["finish_reason"]
                .as_str()
                .map(|s| s.to_string()),
        })
    }

    fn stream(
        &self,
        request: LlmRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>> {
        let client = Arc::clone(&self.client);
        let model = self.model.clone();

        Box::pin(async_stream::stream! {
            let messages = OpenAiProvider::convert_messages(&request.messages);

            let body = json!({
                "model": model,
                "messages": messages,
                "temperature": request.temperature,
                "max_tokens": request.max_tokens,
                "stream": true
            });

            let stream_result: Result<Pin<Box<dyn Stream<Item = Result<Value, _>> + Send>>, _> =
                client.chat().create_stream_byot(body).await;

            let mut stream = match stream_result {
                Ok(s) => s,
                Err(e) => {
                    yield Err(LlmError::ProviderError {
                        status: 500,
                        body: e.to_string(),
                    });
                    return;
                }
            };

            let mut index: u32 = 0;
            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        let content = chunk["choices"][0]["delta"]["content"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        if !content.is_empty() {
                            yield Ok(StreamChunk {
                                content,
                                index,
                                finish_reason: chunk["choices"][0]["finish_reason"]
                                    .as_str()
                                    .map(|s| s.to_string()),
                            });
                            index += 1;
                        }
                    }
                    Err(e) => {
                        yield Err(LlmError::ProviderError {
                            status: 500,
                            body: e.to_string(),
                        });
                        return;
                    }
                }
            }
        })
    }
}
