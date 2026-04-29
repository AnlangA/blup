use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone)]
pub struct LlmClient {
    http: Client,
    gateway_url: String,
    gateway_secret: String,
}

#[derive(Debug, Serialize)]
pub struct GatewayRequest {
    pub model: String,
    pub messages: Vec<GatewayMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatewayMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct GatewayResponse {
    pub content: String,
    pub model: String,
    pub provider: String,
    pub usage: GatewayUsage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GatewayUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Gateway error {status}: {body}")]
    GatewayError { status: u16, body: String },

    #[error("Stream ended unexpectedly")]
    StreamEnded,

    #[error("Response validation failed: {0}")]
    Validation(String),

    #[error("Gateway unavailable: {0}")]
    GatewayUnavailable(String),

    #[error("JSON parse error: {0}")]
    JsonParse(String),
}

impl LlmClient {
    pub fn new(gateway_url: String, gateway_secret: String) -> Self {
        Self {
            http: Client::new(),
            gateway_url,
            gateway_secret,
        }
    }

    pub async fn complete(&self, request: GatewayRequest) -> Result<GatewayResponse, LlmError> {
        tracing::debug!(
            model = %request.model,
            messages_count = request.messages.len(),
            "Sending request to LLM Gateway"
        );

        let response = self
            .http
            .post(format!("{}/v1/gateway/complete", self.gateway_url))
            .header("X-Gateway-Secret", &self.gateway_secret)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to connect to Gateway");
                LlmError::GatewayUnavailable(e.to_string())
            })?;

        let status = response.status().as_u16();
        tracing::debug!(status = status, "Received response from Gateway");

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!(status = status, body = %body, "Gateway returned error");
            // Try to parse error message from JSON response
            let error_msg = if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                json.get("detail")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&body)
                    .to_string()
            } else if body.is_empty() {
                format!("Gateway returned status {status}")
            } else {
                body
            };
            return Err(LlmError::GatewayError {
                status,
                body: error_msg,
            });
        }

        let response_text = response.text().await?;
        let truncated = if response_text.len() > 500 {
            // Find a valid char boundary at or before 500
            let mut end = 500.min(response_text.len());
            while end > 0 && !response_text.is_char_boundary(end) {
                end -= 1;
            }
            &response_text[..end]
        } else {
            &response_text
        };
        tracing::debug!(response = %truncated, "Gateway response body");

        serde_json::from_str(&response_text).map_err(|e| {
            tracing::error!(error = %e, response = %truncated, "Failed to parse Gateway response");
            LlmError::JsonParse(e.to_string())
        })
    }
}
