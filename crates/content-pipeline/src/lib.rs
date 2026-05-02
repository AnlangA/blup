pub mod citation;
pub mod error;
pub mod export;
pub mod import;
pub mod models;

use std::path::Path;

use error::{ExportError, ImportError};
use models::SourceDocument;

/// Content pipeline for importing and exporting learning materials
pub struct ContentPipeline {
    renderer: export::TypstRenderer,
}

impl ContentPipeline {
    pub fn new() -> Self {
        Self {
            renderer: export::TypstRenderer::new(),
        }
    }

    /// Import a file (auto-detect type by extension)
    pub async fn import_file(&self, path: &Path) -> Result<SourceDocument, ImportError> {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "pdf" => import::import_pdf(path).await,
            "md" | "markdown" => import::import_markdown(path).await,
            "txt" | "text" => import::import_text(path).await,
            _ => Err(ImportError::UnsupportedType {
                extension: extension.clone(),
            }),
        }
    }

    /// Import from a URL
    pub async fn import_website(&self, url: &str) -> Result<SourceDocument, ImportError> {
        import::import_website(url).await
    }

    /// Render a chapter to Typst
    pub fn render_chapter_to_typst(
        &self,
        chapter: &serde_json::Value,
    ) -> Result<String, ExportError> {
        self.renderer.render_chapter(chapter)
    }

    /// Render a curriculum to Typst
    pub fn render_curriculum_to_typst(
        &self,
        curriculum: &serde_json::Value,
    ) -> Result<String, ExportError> {
        self.renderer.render_curriculum(curriculum)
    }
}

impl Default for ContentPipeline {
    fn default() -> Self {
        Self::new()
    }
}
