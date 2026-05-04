use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileAnswer {
    pub question_id: String,
    pub answer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionRequest {
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum SseEvent {
    #[serde(rename = "chunk")]
    Chunk { content: String, index: u32 },
    #[serde(rename = "status")]
    Status { state: String, message: String },
    #[serde(rename = "error")]
    Error { code: String, message: String },
    #[serde(rename = "done")]
    Done { result: serde_json::Value },
    #[serde(rename = "ping")]
    Ping {},
    #[serde(rename = "stdout")]
    Stdout { content: String },
    #[serde(rename = "stderr")]
    Stderr { content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxExecuteRequest {
    pub session_id: String,
    pub language: String,
    pub code: String,
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

pub type InteractiveStartRequest = SandboxExecuteRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveStartResponse {
    pub interactive_id: String,
    pub container_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InteractiveClientMessage {
    Stdin { data: String },
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InteractiveServerMessage {
    Stdout { data: String },
    Stderr { data: String },
    Exit { code: Option<i32> },
    Error { code: String, message: String },
}
