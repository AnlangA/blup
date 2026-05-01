use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningGoal {
    pub description: String,
    pub domain: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_level: Option<String>,
}

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
}
