use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::types::*;
use super::{LlmError, LlmProvider};
use crate::config::ProviderConfig;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic Claude provider (direct HTTP, reference opencode transform pattern).
pub struct AnthropicProvider {
    http: Client,
    api_key: String,
    base_url: String,
    model: String,
}

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    usage: AnthropicUsage,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

// Streaming types
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStartData },
    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: DeltaBlock },
    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaData },
    #[serde(rename = "message_stop")]
    MessageStop,
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "error")]
    Error { error: ErrorData },
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessageStartData {
    id: String,
    model: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct DeltaBlock {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MessageDeltaData {
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct ErrorData {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

impl AnthropicProvider {
    pub fn new(config: &ProviderConfig) -> Result<Self, LlmError> {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                LlmError::Config("Missing API key for Anthropic provider".to_string())
            })?;

        let base_url = config.base_url.clone().unwrap_or_else(|| {
            ANTHROPIC_API_URL
                .trim_end_matches("/v1/messages")
                .to_string()
        });

        let http = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| LlmError::Config(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            api_key,
            base_url,
            model: config.model.clone(),
        })
    }

    /// Convert unified messages to Anthropic format.
    /// Anthropic requires system message as a separate top-level field.
    fn convert_messages(messages: &[LlmMessage]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system = None;
        let mut anthropic_messages = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    // Anthropic supports concatenated system messages
                    system = Some(match system {
                        Some(existing) => format!("{existing}\n\n{}", msg.content),
                        None => msg.content.clone(),
                    });
                }
                Role::User => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "user".to_string(),
                        content: msg.content.clone(),
                    });
                }
                Role::Assistant => {
                    anthropic_messages.push(AnthropicMessage {
                        role: "assistant".to_string(),
                        content: msg.content.clone(),
                    });
                }
            }
        }

        (system, anthropic_messages)
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let (system, messages) = Self::convert_messages(&request.messages);

        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));

        let body = AnthropicRequest {
            model: self.model.clone(),
            system,
            messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            stream: false,
        };

        let response = self
            .http
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_API_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(LlmError::ProviderError {
                status,
                body: body_text,
            });
        }

        let resp: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LlmError::JsonParse(format!("Failed to parse Anthropic response: {e}")))?;

        let content = resp
            .content
            .into_iter()
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(LlmResponse {
            content,
            model: resp.model,
            provider: "anthropic".to_string(),
            usage: TokenUsage {
                prompt_tokens: resp.usage.input_tokens,
                completion_tokens: resp.usage.output_tokens,
                total_tokens: resp.usage.input_tokens + resp.usage.output_tokens,
            },
            finish_reason: resp.stop_reason,
        })
    }

    fn stream(
        &self,
        request: LlmRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>> {
        let http = self.http.clone();
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = self.model.clone();

        Box::pin(async_stream::stream! {
            let (system, messages) = AnthropicProvider::convert_messages(&request.messages);

            let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

            let body = AnthropicRequest {
                model,
                system,
                messages,
                max_tokens: request.max_tokens.unwrap_or(4096),
                temperature: request.temperature,
                stream: true,
            };

            let response = match http
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", ANTHROPIC_API_VERSION)
                .header("content-type", "application/json")
                .json(&body)
                .send()
                .await
            {
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
            let mut chunk_index: u32 = 0;

            while let Some(result) = byte_stream.next().await {
                match result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        while let Some(pos) = buffer.find('\n') {
                            let line = buffer[..pos].trim().to_string();
                            buffer = buffer[pos + 1..].to_string();

                            if let Some(data_str) = line.strip_prefix("data: ") {
                                if data_str.is_empty() || data_str == "{}" {
                                    continue;
                                }
                                match serde_json::from_str::<StreamEvent>(data_str) {
                                    Ok(StreamEvent::ContentBlockDelta { delta, .. }) => {
                                        if let Some(text) = delta.text {
                                            yield Ok(StreamChunk {
                                                content: text,
                                                index: chunk_index,
                                                finish_reason: None,
                                            });
                                            chunk_index += 1;
                                        }
                                    }
                                    Ok(StreamEvent::Error { error }) => {
                                        yield Err(LlmError::ProviderError {
                                            status: 500,
                                            body: format!("{}: {}", error.error_type, error.message),
                                        });
                                        return;
                                    }
                                    Ok(_) => {} // Ignore other event types
                                    Err(_) => {} // Ignore parse errors for non-JSON lines
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
