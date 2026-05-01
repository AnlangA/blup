pub mod event;
pub mod storage;

use tokio::sync::mpsc;

pub use event::{AuditEvent, AuditEventType};

use self::storage::AuditStorage;
use crate::config::AuditConfig;

/// Audit logger that asynchronously writes events to storage.
pub struct AuditLogger {
    tx: mpsc::UnboundedSender<AuditEvent>,
    enabled: bool,
}

impl AuditLogger {
    /// Create a new audit logger with background writer.
    pub fn new(config: &AuditConfig) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<AuditEvent>();

        if config.enabled {
            let storage = AuditStorage::new(config.storage_dir.clone());
            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    if let Err(e) = storage.append(&event).await {
                        tracing::error!(error = %e, "Failed to write audit event");
                    }
                }
            });
        }

        Self {
            tx,
            enabled: config.enabled,
        }
    }

    /// Log an audit event (non-blocking, sent to background writer).
    pub fn log(&self, event: AuditEvent) {
        if !self.enabled {
            return;
        }
        let _ = self.tx.send(event);
    }

    /// Log an LLM call with timing.
    #[allow(clippy::too_many_arguments)]
    pub fn log_llm_call(
        &self,
        session_id: &str,
        provider: &str,
        model: &str,
        usage: &crate::provider::TokenUsage,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    ) {
        self.log(AuditEvent::llm_call(
            session_id,
            provider,
            model,
            usage,
            duration_ms,
            success,
            error,
        ));
    }

    /// Log a tool call with args hash.
    pub fn log_tool_call(
        &self,
        session_id: &str,
        tool_name: &str,
        args_json: &str,
        duration_ms: u64,
        success: bool,
        error: Option<String>,
    ) {
        let args_hash = storage::hash_content(args_json);
        self.log(AuditEvent::tool_call(
            session_id,
            tool_name,
            &args_hash,
            duration_ms,
            success,
            error,
        ));
    }

    /// Log a state transition.
    pub fn log_state_transition(&self, session_id: &str, from: &str, to: &str, trigger: &str) {
        self.log(AuditEvent::state_transition(session_id, from, to, trigger));
    }

    /// Whether audit logging is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
