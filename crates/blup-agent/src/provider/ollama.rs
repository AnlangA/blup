use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::types::*;
use super::{LlmError, LlmProvider};
use crate::config::ProviderConfig;

const OLLAMA_DEFAULT_URL: &str = "http://localhost:11434";

/// Ollama local model provider.
pub struct OllamaProvider {
    http: Client,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: Option<OllamaResponseMessage>,
    done: bool,
    #[allow(dead_code)]
    total_duration: Option<u64>,
    eval_count: Option<u32>,
    prompt_eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

impl OllamaProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, LlmError> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| OLLAMA_DEFAULT_URL.to_string());

        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Config(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            base_url,
            model: config.model.clone(),
        })
    }

    fn convert_messages(messages: &[LlmMessage]) -> Vec<OllamaMessage> {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "system",
                    Role::User => "user",
                    Role::Assistant => "assistant",
                };
                OllamaMessage {
                    role: role.to_string(),
                    content: m.content.clone(),
                }
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));

        let body = OllamaRequest {
            model: self.model.clone(),
            messages: Self::convert_messages(&request.messages),
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        let response = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| LlmError::Unavailable(e.to_string()))?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError {
                status,
                body: body_text,
            });
        }

        let resp: OllamaResponse = response
            .json()
            .await
            .map_err(|e| LlmError::JsonParse(format!("Failed to parse Ollama response: {e}")))?;

        let content = resp.message.map(|m| m.content).unwrap_or_default();

        Ok(LlmResponse {
            content,
            model: self.model.clone(),
            provider: "ollama".to_string(),
            usage: TokenUsage {
                prompt_tokens: resp.prompt_eval_count.unwrap_or(0),
                completion_tokens: resp.eval_count.unwrap_or(0),
                total_tokens: resp.prompt_eval_count.unwrap_or(0) + resp.eval_count.unwrap_or(0),
            },
            finish_reason: if resp.done {
                Some("stop".to_string())
            } else {
                None
            },
        })
    }

    fn stream(
        &self,
        request: LlmRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>> {
        let http = self.http.clone();
        let base_url = self.base_url.clone();
        let model = self.model.clone();

        Box::pin(async_stream::stream! {
            let url = format!("{}/api/chat", base_url.trim_end_matches('/'));

            let body = OllamaRequest {
                model,
                messages: OllamaProvider::convert_messages(&request.messages),
                stream: true,
                options: OllamaOptions {
                    temperature: request.temperature,
                    num_predict: request.max_tokens,
                },
            };

            let response = match http.post(&url).json(&body).send().await {
                Ok(r) => r,
                Err(e) => {
                    yield Err(LlmError::Unavailable(e.to_string()));
                    return;
                }
            };

            let status = response.status().as_u16();
            if !response.status().is_success() {
                let body_text = response.text().await.unwrap_or_default();
                yield Err(LlmError::ProviderError { status, body: body_text });
                return;
            }

            let mut byte_stream = response.bytes_stream();
            let mut buffer = String::new();
            let mut index: u32 = 0;

            while let Some(result) = byte_stream.next().await {
                match result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].to_string();
                            buffer = buffer[pos + 1..].to_string();

                            if line.trim().is_empty() {
                                continue;
                            }

                            if let Ok(resp) = serde_json::from_str::<OllamaResponse>(&line) {
                                if let Some(msg) = resp.message {
                                    yield Ok(StreamChunk {
                                        content: msg.content,
                                        index,
                                        finish_reason: if resp.done {
                                            Some("stop".to_string())
                                        } else {
                                            None
                                        },
                                    });
                                    index += 1;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(LlmError::Http(e));
                        return;
                    }
                }
            }
        })
    }
}
