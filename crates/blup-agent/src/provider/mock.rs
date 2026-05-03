use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::Stream;

use super::types::*;
use super::{LlmError, LlmProvider};

/// Mock provider for testing. Returns deterministic responses.
pub struct MockProvider {
    responses: Mutex<VecDeque<String>>,
    call_log: Mutex<Vec<LlmRequest>>,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            responses: Mutex::new(VecDeque::new()),
            call_log: Mutex::new(Vec::new()),
        }
    }

    /// Add a mock response to the back of the queue.
    pub fn push_response(&self, response: impl Into<String>) {
        self.responses.lock().unwrap().push_back(response.into());
    }

    /// Replace a response at the given index. Panics if index is out of bounds.
    pub fn replace_response(&self, index: usize, response: impl Into<String>) {
        let mut q = self.responses.lock().unwrap();
        if index < q.len() {
            q[index] = response.into();
        }
    }

    /// Get recorded calls for assertions.
    pub fn call_log(&self) -> Vec<LlmRequest> {
        self.call_log.lock().unwrap().clone()
    }

    /// Create with default learning-flow responses.
    pub fn with_default_responses() -> Self {
        let mock = Self::new();
        mock.push_response(r#"{"feasible":true,"reason":"This is a well-defined learning goal.","suggestions":["Start with fundamentals","Practice regularly"],"estimated_duration":"4 weeks","prerequisites":["Basic computer skills"]}"#);
        mock.push_response(r#"{"next_question":"What learning format works best for you?"}"#);
        mock.push_response(r#"{"next_question":"How much time can you dedicate each week?"}"#);
        mock.push_response(r#"{"experience_level":{"domain_knowledge":"beginner"},"learning_style":{"preferred_format":["text","interactive"],"pace_preference":"moderate"},"available_time":{"hours_per_week":10,"preferred_session_length_minutes":60}}"#);
        mock.push_response(r#"{"title":"Learning Plan","description":"A structured curriculum","chapters":[{"id":"ch1","title":"Introduction","order":1,"objectives":["Understand basics"],"estimated_minutes":30,"prerequisites":[]},{"id":"ch2","title":"Core Concepts","order":2,"objectives":["Master fundamentals"],"estimated_minutes":45,"prerequisites":["ch1"]}],"estimated_duration":"3 weeks","learning_objectives":["Understand basics","Master fundamentals"]}"#);
        mock.push_response("# Chapter 1: Introduction\n\nWelcome to this chapter. Here we will explore the fundamental concepts...");
        mock.push_response("Great question! A variable is a named storage location in memory that holds a value...");
        mock
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        "mock-model"
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.call_log.lock().unwrap().push(request.clone());

        let content = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| r#"{"mock":true}"#.to_string());

        Ok(LlmResponse {
            content,
            model: "mock-model".to_string(),
            provider: "mock".to_string(),
            usage: TokenUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
            },
            finish_reason: Some("stop".to_string()),
        })
    }

    fn stream(
        &self,
        request: LlmRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>> {
        self.call_log.lock().unwrap().push(request.clone());

        let content = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| r#"{"mock":true}"#.to_string());

        Box::pin(async_stream::stream! {
            // Simulate streaming in 16-byte chunks
            let bytes = content.as_bytes();
            let mut offset = 0;
            let mut index: u32 = 0;
            while offset < bytes.len() {
                let end = (offset + 16).min(bytes.len());
                let chunk = String::from_utf8_lossy(&bytes[offset..end]).to_string();
                yield Ok(StreamChunk {
                    content: chunk,
                    index,
                    finish_reason: None,
                });
                offset = end;
                index += 1;
            }
            yield Ok(StreamChunk {
                content: String::new(),
                index,
                finish_reason: Some("stop".to_string()),
            });
        })
    }
}
