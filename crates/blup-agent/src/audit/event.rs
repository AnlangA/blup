use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::provider::TokenUsage;

/// A single audit event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub event_type: AuditEventType,
}

/// Types of auditable events.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AuditEventType {
    /// LLM API call completed.
    LlmCall {
        provider: String,
        model: String,
        prompt_tokens: u32,
        completion_tokens: u32,
        total_tokens: u32,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    },
    /// Tool execution.
    ToolCall {
        tool_name: String,
        args_hash: String,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    },
    /// State machine transition.
    StateTransition {
        from_state: String,
        to_state: String,
        trigger: String,
    },
    /// MCP server connection event.
    McpConnection {
        server_name: String,
        status: String,
        transport: String,
    },
    /// Session lifecycle event.
    SessionEvent {
        action: String,
        details: Option<String>,
    },
    /// Error occurred.
    Error {
        code: String,
        message: String,
        context: Option<String>,
    },
    /// Memory compaction performed.
    MemoryCompaction {
        messages_before: usize,
        messages_after: usize,
        tokens_saved: usize,
    },
}

impl AuditEvent {
    pub fn new(session_id: impl Into<String>, event_type: AuditEventType) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            session_id: session_id.into(),
            event_type,
        }
    }

    /// Create an LLM call audit event with auto-redacted details.
    pub fn llm_call(
        session_id: &str,
        provider: &str,
        model: &str,
        usage: &TokenUsage,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    ) -> Self {
        Self::new(
            session_id,
            AuditEventType::LlmCall {
                provider: provider.to_string(),
                model: model.to_string(),
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
                duration_ms,
                success,
                error,
            },
        )
    }

    /// Create a tool call audit event.
    pub fn tool_call(
        session_id: &str,
        tool_name: &str,
        args_hash: &str,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    ) -> Self {
        Self::new(
            session_id,
            AuditEventType::ToolCall {
                tool_name: tool_name.to_string(),
                args_hash: args_hash.to_string(),
                duration_ms,
                success,
                error,
            },
        )
    }

    /// Create a state transition audit event.
    pub fn state_transition(session_id: &str, from: &str, to: &str, trigger: &str) -> Self {
        Self::new(
            session_id,
            AuditEventType::StateTransition {
                from_state: from.to_string(),
                to_state: to.to_string(),
                trigger: trigger.to_string(),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_new() {
        let event = AuditEvent::new(
            "session-123",
            AuditEventType::SessionEvent {
                action: "created".to_string(),
                details: None,
            },
        );
        assert_eq!(event.session_id, "session-123");
        assert!(!event.id.is_empty());
    }

    #[test]
    fn test_llm_call_event() {
        let usage = TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };
        let event = AuditEvent::llm_call("sess", "openai", "gpt-4", &usage, 500, true, None);
        assert_eq!(event.session_id, "sess");
        match event.event_type {
            AuditEventType::LlmCall {
                provider,
                model,
                total_tokens,
                success,
                ..
            } => {
                assert_eq!(provider, "openai");
                assert_eq!(model, "gpt-4");
                assert_eq!(total_tokens, 150);
                assert!(success);
            }
            _ => panic!("Expected LlmCall event"),
        }
    }

    #[test]
    fn test_tool_call_event() {
        let event = AuditEvent::tool_call("sess", "web_search", "abc123", 100, true, None);
        match event.event_type {
            AuditEventType::ToolCall {
                tool_name,
                args_hash,
                success,
                ..
            } => {
                assert_eq!(tool_name, "web_search");
                assert_eq!(args_hash, "abc123");
                assert!(success);
            }
            _ => panic!("Expected ToolCall event"),
        }
    }

    #[test]
    fn test_state_transition_event() {
        let event = AuditEvent::state_transition("sess", "IDLE", "GOAL_INPUT", "SubmitGoal");
        match event.event_type {
            AuditEventType::StateTransition {
                from_state,
                to_state,
                trigger,
            } => {
                assert_eq!(from_state, "IDLE");
                assert_eq!(to_state, "GOAL_INPUT");
                assert_eq!(trigger, "SubmitGoal");
            }
            _ => panic!("Expected StateTransition event"),
        }
    }

    #[test]
    fn test_event_serialization() {
        let event = AuditEvent::state_transition("sess", "A", "B", "trigger");
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: AuditEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, "sess");
    }
}
