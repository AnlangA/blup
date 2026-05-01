use super::types::*;

/// Provider-specific message transformations.
///
/// Different LLM providers have different requirements:
/// - Anthropic: system message must be a top-level field, not in messages array
/// - OpenAI: system messages go in the messages array
/// - Some providers don't support certain message types
pub struct ProviderTransform;

impl ProviderTransform {
    /// Merge consecutive messages with the same role (some providers require this).
    pub fn merge_consecutive(messages: &[LlmMessage]) -> Vec<LlmMessage> {
        let mut merged: Vec<LlmMessage> = Vec::new();
        for msg in messages {
            if let Some(last) = merged.last_mut() {
                if last.role == msg.role {
                    last.content.push_str("\n\n");
                    last.content.push_str(&msg.content);
                    continue;
                }
            }
            merged.push(msg.clone());
        }
        merged
    }

    /// Ensure there's at least one user message (required by all providers).
    pub fn ensure_user_message(messages: &[LlmMessage]) -> Vec<LlmMessage> {
        let has_user = messages.iter().any(|m| m.role == Role::User);
        if has_user {
            messages.to_vec()
        } else {
            let mut result = messages.to_vec();
            result.push(LlmMessage::user("Please continue."));
            result
        }
    }

    /// Truncate messages to fit within a token budget (rough estimation).
    /// Uses a simple heuristic: ~4 characters per token.
    pub fn truncate_to_budget(messages: &[LlmMessage], max_tokens: usize) -> Vec<LlmMessage> {
        let max_chars = max_tokens * 4;
        let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();

        if total_chars <= max_chars {
            return messages.to_vec();
        }

        // Keep system message and recent messages, drop from the middle
        let mut result: Vec<LlmMessage> = Vec::new();
        let mut char_budget = max_chars;

        // Always keep system messages
        for msg in messages.iter().filter(|m| m.role == Role::System) {
            char_budget = char_budget.saturating_sub(msg.content.len());
            result.push(msg.clone());
        }

        // Add messages from the end, skipping middle ones if needed
        let non_system: Vec<&LlmMessage> =
            messages.iter().filter(|m| m.role != Role::System).collect();

        let mut from_end = Vec::new();
        for msg in non_system.iter().rev() {
            if char_budget >= msg.content.len() {
                char_budget -= msg.content.len();
                from_end.insert(0, (*msg).clone());
            } else {
                break;
            }
        }

        result.extend(from_end);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_consecutive() {
        let messages = vec![
            LlmMessage::user("Hello"),
            LlmMessage::user("World"),
            LlmMessage::assistant("Hi"),
        ];
        let merged = ProviderTransform::merge_consecutive(&messages);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].content, "Hello\n\nWorld");
    }

    #[test]
    fn test_ensure_user_message() {
        let messages = vec![LlmMessage::system("You are helpful.")];
        let result = ProviderTransform::ensure_user_message(&messages);
        assert_eq!(result.len(), 2);
        assert_eq!(result[1].role, Role::User);
    }

    #[test]
    fn test_truncate_to_budget() {
        let messages = vec![
            LlmMessage::system("Short system"),
            LlmMessage::user("A".repeat(1000)),
            LlmMessage::assistant("B".repeat(1000)),
            LlmMessage::user("Recent message"),
        ];
        let result = ProviderTransform::truncate_to_budget(&messages, 100);
        // Should keep system + recent, drop the large middle messages
        assert!(result.len() < messages.len());
    }
}
