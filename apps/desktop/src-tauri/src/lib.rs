use std::sync::Arc;

use content_pipeline::ContentPipeline;

pub mod commands;

pub struct AppState {
    pub content_pipeline: Arc<ContentPipeline>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        Self {
            content_pipeline: Arc::new(ContentPipeline::new()),
        }
    }
}
