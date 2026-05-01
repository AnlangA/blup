use std::collections::VecDeque;

use crate::provider::LlmMessage;

/// Short-term memory: manages the current conversation context window.
pub struct ShortTermMemory {
    messages: VecDeque<LlmMessage>,
    max_tokens: usize,
    current_chars: usize,
}

impl ShortTermMemory {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_tokens,
            current_chars: 0,
        }
    }

    /// Add a message to the context.
    pub fn push(&mut self, message: LlmMessage) {
        self.current_chars += message.content.len();
        self.messages.push_back(message);
    }

    /// Get all messages as a Vec (for passing to LLM).
    pub fn messages(&self) -> Vec<LlmMessage> {
        self.messages.iter().cloned().collect()
    }

    /// Check if context is over the token budget.
    /// Uses ~4 chars per token as estimation.
    pub fn is_over_budget(&self) -> bool {
        let estimated_tokens = self.current_chars / 4;
        estimated_tokens > self.max_tokens
    }

    /// Estimate current token count.
    pub fn estimated_tokens(&self) -> usize {
        self.current_chars / 4
    }

    /// Remove the oldest non-system message and return it.
    pub fn pop_oldest(&mut self) -> Option<LlmMessage> {
        // Find the first non-system message
        let idx = self
            .messages
            .iter()
            .position(|m| m.role != crate::provider::Role::System)?;

        let msg = self.messages.remove(idx)?;
        self.current_chars -= msg.content.len();
        Some(msg)
    }

    /// Get number of messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Clear all messages.
    pub fn clear(&mut self) {
        self.messages.clear();
        self.current_chars = 0;
    }

    /// Get messages from the last N turns (user+assistant pairs).
    pub fn recent_turns(&self, n: usize) -> Vec<LlmMessage> {
        let non_system: Vec<&LlmMessage> = self
            .messages
            .iter()
            .filter(|m| m.role != crate::provider::Role::System)
            .collect();

        // Each turn is roughly a user + assistant pair
        let take = n * 2;
        let start = non_system.len().saturating_sub(take);
        non_system[start..].iter().map(|m| (*m).clone()).collect()
    }

    /// Get system messages.
    pub fn system_messages(&self) -> Vec<LlmMessage> {
        self.messages
            .iter()
            .filter(|m| m.role == crate::provider::Role::System)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_messages() {
        let mut mem = ShortTermMemory::new(1000);
        mem.push(LlmMessage::system("You are helpful."));
        mem.push(LlmMessage::user("Hello"));
        mem.push(LlmMessage::assistant("Hi there!"));

        assert_eq!(mem.len(), 3);
        assert_eq!(mem.messages().len(), 3);
    }

    #[test]
    fn test_pop_oldest_skips_system() {
        let mut mem = ShortTermMemory::new(1000);
        mem.push(LlmMessage::system("System prompt"));
        mem.push(LlmMessage::user("Hello"));
        mem.push(LlmMessage::assistant("Hi"));

        let popped = mem.pop_oldest().unwrap();
        assert_eq!(popped.role, crate::provider::Role::User);
        assert_eq!(mem.len(), 2);
    }

    #[test]
    fn test_recent_turns() {
        let mut mem = ShortTermMemory::new(1000);
        mem.push(LlmMessage::system("System"));
        mem.push(LlmMessage::user("Q1"));
        mem.push(LlmMessage::assistant("A1"));
        mem.push(LlmMessage::user("Q2"));
        mem.push(LlmMessage::assistant("A2"));

        let recent = mem.recent_turns(1);
        assert_eq!(recent.len(), 2); // Last user+assistant pair
    }
}
