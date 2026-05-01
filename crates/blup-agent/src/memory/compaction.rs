use std::sync::Arc;

use crate::provider::{LlmMessage, LlmProvider, LlmRequest, Role};

/// Context compactor: summarizes old messages to reduce context window usage.
///
/// Reference: opencode's compaction strategy — keeps recent turns verbatim,
/// summarizes older conversation into structured sections.
pub struct Compactor {
    provider: Arc<dyn LlmProvider>,
}

/// Result of a compaction operation.
#[derive(Debug)]
pub struct CompactionResult {
    /// The summary message that replaces old messages.
    pub summary: LlmMessage,
    /// Number of messages removed.
    pub messages_removed: usize,
    /// Estimated tokens saved.
    pub tokens_saved: usize,
}

impl Compactor {
    pub fn new(provider: Arc<dyn LlmProvider>) -> Self {
        Self { provider }
    }

    /// Compact a conversation by summarizing older messages.
    ///
    /// Keeps the last `keep_recent` turns verbatim, summarizes the rest.
    pub async fn compact(
        &self,
        messages: &[LlmMessage],
        keep_recent: usize,
    ) -> Result<CompactionResult, crate::error::AgentError> {
        let non_system: Vec<&LlmMessage> =
            messages.iter().filter(|m| m.role != Role::System).collect();

        if non_system.len() <= keep_recent * 2 {
            // Not enough messages to compact
            return Ok(CompactionResult {
                summary: LlmMessage::system("No compaction needed."),
                messages_removed: 0,
                tokens_saved: 0,
            });
        }

        let split_point = non_system.len().saturating_sub(keep_recent * 2);
        let to_summarize = &non_system[..split_point];
        let _to_keep = &non_system[split_point..];

        let conversation_text: String = to_summarize
            .iter()
            .map(|m| {
                let role = match m.role {
                    Role::System => "System",
                    Role::User => "User",
                    Role::Assistant => "Assistant",
                };
                format!("{role}: {}", m.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let summary_prompt = format!(
            "Summarize the following conversation into a structured context document. \
             Include these sections:\n\
             ## Goal\n## Progress\n## Key Decisions\n## Next Steps\n## Critical Context\n\n\
             Conversation:\n{conversation_text}"
        );

        let request = LlmRequest {
            model: self.provider.model().to_string(),
            messages: vec![
                LlmMessage::system(
                    "You are a conversation summarizer. Create concise, structured summaries.\
                     Preserve important facts, decisions, and context.",
                ),
                LlmMessage::user(summary_prompt),
            ],
            temperature: Some(0.2),
            max_tokens: Some(2048),
            stream: false,
        };

        let response = self.provider.complete(request).await?;

        let tokens_saved = to_summarize
            .iter()
            .map(|m| m.content.len() / 4)
            .sum::<usize>()
            .saturating_sub(response.content.len() / 4);

        Ok(CompactionResult {
            summary: LlmMessage::system(format!("[Compacted Context]\n\n{}", response.content)),
            messages_removed: to_summarize.len(),
            tokens_saved,
        })
    }
}
