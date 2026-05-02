use std::sync::Arc;

use content_pipeline::ContentPipeline;

pub mod commands;

pub struct AppState {
    pub content_pipeline: Arc<ContentPipeline>,
    pub agent_core_url: String,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            content_pipeline: Arc::new(ContentPipeline::new()),
            agent_core_url: "http://localhost:3000".to_string(),
        }
    }
}
