use std::sync::Arc;

use content_pipeline::ContentPipeline;

pub mod commands;

pub struct AppState {
    pub content_pipeline: Arc<ContentPipeline>,
    pub sandbox_manager: Arc<sandbox_manager::SandboxManager>,
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
            sandbox_manager: Arc::new(sandbox_manager::SandboxManager::new(
                sandbox_manager::SandboxConfig::default(),
            )),
        }
    }
}
