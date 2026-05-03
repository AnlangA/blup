use serde::{Deserialize, Serialize};

/// Result of a single agent step (feasibility check, profile round, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult<T> {
    pub data: T,
    pub model: String,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Events emitted during streaming agent operations.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum AgentStreamEvent {
    #[serde(rename = "status")]
    Status { state: String, message: String },
    #[serde(rename = "chunk")]
    Chunk { content: String, index: u32 },
    #[serde(rename = "error")]
    Error { code: String, message: String },
    #[serde(rename = "done")]
    Done { result: serde_json::Value },
}

/// Context for feasibility check.
#[derive(Debug, Clone)]
pub struct FeasibilityContext {
    pub learning_goal: String,
    pub domain: String,
    pub context: Option<String>,
}

/// Context for profile collection.
#[derive(Debug, Clone)]
pub struct ProfileContext {
    pub learning_goal: String,
    pub domain: String,
    pub answer: String,
    pub round: u32,
    pub total_rounds: u32,
    pub is_final: bool,
    pub profile_history: serde_json::Value,
}

/// Context for curriculum generation.
#[derive(Debug, Clone)]
pub struct CurriculumContext {
    pub learning_goal: String,
    pub profile: serde_json::Value,
}

/// Context for chapter teaching.
#[derive(Debug, Clone)]
pub struct ChapterContext {
    pub chapter_id: String,
    pub chapter_title: String,
    pub profile: serde_json::Value,
    pub curriculum_context: serde_json::Value,
}

/// Context for repairing invalid chapter Markdown.
#[derive(Debug, Clone)]
pub struct ChapterMarkdownRepairContext {
    pub chapter_id: String,
    pub chapter_title: String,
    pub original_markdown: String,
    pub issues: Vec<String>,
}

/// Context for Q&A within a chapter.
#[derive(Debug, Clone)]
pub struct QaContext {
    pub question: String,
    pub chapter_content: String,
    pub profile: serde_json::Value,
    pub conversation_history: serde_json::Value,
    pub curriculum_context: serde_json::Value,
}

/// Outcome of a profile step.
#[derive(Debug, Clone)]
pub enum ProfileStep {
    Intermediate {
        round: u32,
        total_rounds: u32,
        next_question_hint: String,
    },
    Complete {
        profile: serde_json::Value,
    },
}
