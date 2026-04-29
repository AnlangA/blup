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
        let response = self
            .http
            .post(format!("{}/v1/gateway/complete", self.gateway_url))
            .header("X-Gateway-Secret", &self.gateway_secret)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::GatewayError { status, body });
        }

        Ok(response.json().await?)
    }
}
