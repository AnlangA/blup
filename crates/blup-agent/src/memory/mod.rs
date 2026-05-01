pub mod compaction;
pub mod long_term;
pub mod short_term;

use std::sync::Arc;

use crate::config::MemoryConfig;
use crate::provider::{LlmMessage, LlmProvider};

pub use compaction::{CompactionResult, Compactor};
pub use long_term::{LongTermMemory, SessionSummary};
pub use short_term::ShortTermMemory;

/// Manages both short-term and long-term memory for an agent session.
pub struct MemoryManager {
    short_term: ShortTermMemory,
    long_term: LongTermMemory,
    compactor: Option<Compactor>,
    keep_recent_turns: usize,
}

impl MemoryManager {
    pub fn new(config: &MemoryConfig, provider: Option<Arc<dyn LlmProvider>>) -> Self {
        let compactor = provider.map(Compactor::new);
        Self {
            short_term: ShortTermMemory::new(config.max_context_tokens),
            long_term: LongTermMemory::new(config.storage_dir.clone()),
            compactor,
            keep_recent_turns: 4,
        }
    }

    /// Add a message to short-term memory.
    pub fn push_message(&mut self, message: LlmMessage) {
        self.short_term.push(message);
    }

    /// Get all current messages for LLM context.
    pub fn messages(&self) -> Vec<LlmMessage> {
        self.short_term.messages()
    }

    /// Check if compaction is needed and perform it if possible.
    pub async fn maybe_compact(&mut self, _session_id: &str) -> Option<CompactionResult> {
        if !self.short_term.is_over_budget() {
            return None;
        }

        let compactor = self.compactor.as_ref()?;
        let messages = self.short_term.messages();

        let result = compactor
            .compact(&messages, self.keep_recent_turns)
            .await
            .ok()?;

        if result.messages_removed == 0 {
            return None;
        }

        // Rebuild short-term memory: system + compaction summary + recent turns
        self.short_term.clear();
        self.short_term.push(result.summary.clone());

        // Re-add recent non-system messages
        let non_system: Vec<&LlmMessage> = messages
            .iter()
            .filter(|m| m.role != crate::provider::Role::System)
            .collect();
        let keep_from = non_system.len().saturating_sub(self.keep_recent_turns * 2);
        for msg in &non_system[keep_from..] {
            self.short_term.push((*msg).clone());
        }

        Some(result)
    }

    /// Load long-term memory for a session.
    pub async fn load_session_summary(&mut self, session_id: &str) -> Option<SessionSummary> {
        self.long_term.load(session_id).await
    }

    /// Save long-term memory for a session.
    pub async fn save_session_summary(
        &mut self,
        summary: &SessionSummary,
    ) -> Result<(), std::io::Error> {
        self.long_term.save(summary).await
    }

    /// Get estimated current token count.
    pub fn estimated_tokens(&self) -> usize {
        self.short_term.estimated_tokens()
    }

    /// Get message count.
    pub fn message_count(&self) -> usize {
        self.short_term.len()
    }

    /// Clear short-term memory.
    pub fn clear_short_term(&mut self) {
        self.short_term.clear();
    }
}
