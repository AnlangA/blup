pub mod citation;
pub mod error;
pub mod export;
pub mod import;
pub mod models;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use error::{ExportError, ImportError};
use export::MarkdownValidationError;
use models::SourceDocument;
use models::{ExportJob, ImportJob};
use tokio::sync::Mutex;
use uuid::Uuid;

/// Content pipeline for importing and exporting learning materials
pub struct ContentPipeline {
    renderer: export::TypstRenderer,
    jobs: Arc<Mutex<PipelineJobs>>,
}

#[derive(Default)]
struct PipelineJobs {
    import_jobs: std::collections::HashMap<Uuid, ImportJob>,
    export_jobs: std::collections::HashMap<Uuid, ExportJob>,
}

impl ContentPipeline {
    pub fn new() -> Self {
        Self {
            renderer: export::TypstRenderer::new(),
            jobs: Arc::new(Mutex::new(PipelineJobs::default())),
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

    pub async fn import_file_job(&self, path: PathBuf) -> Uuid {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let mut job = match extension.as_str() {
            "pdf" => ImportJob::new_pdf(&path.to_string_lossy()),
            "md" | "markdown" => ImportJob::new_markdown(&path.to_string_lossy()),
            _ => ImportJob::new_text(&path.to_string_lossy()),
        };
        let job_id = job.id;
        job.status = models::ImportStatus::Extracting;
        self.jobs.lock().await.import_jobs.insert(job_id, job);
        job_id
    }

    pub async fn import_website_job(&self, url: &str) -> Uuid {
        let mut job = ImportJob::new_website(url);
        let job_id = job.id;
        job.status = models::ImportStatus::Extracting;
        self.jobs.lock().await.import_jobs.insert(job_id, job);
        job_id
    }

    pub async fn get_import_job(&self, job_id: Uuid) -> Option<ImportJob> {
        self.jobs.lock().await.import_jobs.get(&job_id).cloned()
    }

    pub async fn upsert_import_job(&self, job: ImportJob) {
        self.jobs.lock().await.import_jobs.insert(job.id, job);
    }

    pub async fn get_export_job(&self, job_id: Uuid) -> Option<ExportJob> {
        self.jobs.lock().await.export_jobs.get(&job_id).cloned()
    }

    pub async fn upsert_export_job(&self, job: ExportJob) {
        self.jobs.lock().await.export_jobs.insert(job.id, job);
    }

    /// Render a chapter to Typst
    pub fn render_chapter_to_typst(
        &self,
        chapter: &serde_json::Value,
    ) -> Result<String, ExportError> {
        self.renderer.render_chapter(chapter)
    }

    /// Validate generated chapter Markdown before persistence or export.
    pub fn validate_chapter_markdown(&self, markdown: &str) -> Result<(), MarkdownValidationError> {
        export::validate_chapter_markdown(markdown)
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
