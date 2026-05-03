use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{command, AppHandle, Emitter, State};

use crate::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportResult {
    pub doc_id: String,
    pub title: String,
    pub source_type: String,
    pub checksum: String,
    pub chunks: u32,
    pub word_count: u32,
    pub language: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportJobResult {
    pub job_id: String,
}

#[derive(Debug, Serialize)]
pub struct ImportError {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<content_pipeline::error::ImportError> for ImportError {
    fn from(err: content_pipeline::error::ImportError) -> Self {
        ImportError {
            code: "IMPORT_ERROR".to_string(),
            message: err.to_string(),
        }
    }
}

#[command]
pub async fn import_file(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ImportResult, ImportError> {
    // Open native file dialog
    use tauri_plugin_dialog::DialogExt;

    let file_path = app
        .dialog()
        .file()
        .add_filter("Documents", &["pdf", "md", "txt", "markdown"])
        .add_filter("PDF Files", &["pdf"])
        .add_filter("Markdown Files", &["md", "markdown"])
        .add_filter("Text Files", &["txt"])
        .blocking_pick_file();

    let file_path = match file_path {
        Some(path) => PathBuf::from(path.to_string()),
        None => {
            return Err(ImportError {
                code: "USER_CANCELLED".to_string(),
                message: "User cancelled file selection".to_string(),
            })
        }
    };

    // Validate file size (reject > 50MB)
    let metadata = tokio::fs::metadata(&file_path)
        .await
        .map_err(|e| ImportError {
            code: "FILE_ERROR".to_string(),
            message: e.to_string(),
        })?;

    if metadata.len() > 50 * 1024 * 1024 {
        return Err(ImportError {
            code: "FILE_TOO_LARGE".to_string(),
            message: format!("File too large: {} bytes (max 50MB)", metadata.len()),
        });
    }

    // Emit progress
    let _ = app.emit(
        "import:progress",
        serde_json::json!({
            "stage": "extracting",
            "path": file_path.to_string_lossy()
        }),
    );

    // Import via content pipeline
    let job_id = state
        .content_pipeline
        .import_file_job(file_path.clone())
        .await;
    let source_doc = state.content_pipeline.import_file(&file_path).await?;
    if let Some(mut job) = state.content_pipeline.get_import_job(job_id).await {
        job.mark_completed(source_doc.id);
        state.content_pipeline.upsert_import_job(job).await;
    }

    // Emit completion
    let _ = app.emit(
        "import:complete",
        serde_json::json!({
            "doc_id": source_doc.id.to_string(),
            "title": source_doc.title,
            "chunks": source_doc.chunks.len(),
            "word_count": source_doc.metadata.word_count
        }),
    );

    Ok(ImportResult {
        doc_id: source_doc.id.to_string(),
        title: source_doc.title,
        source_type: source_doc.source_type.to_string(),
        checksum: source_doc.checksum,
        chunks: source_doc.chunks.len() as u32,
        word_count: source_doc.metadata.word_count,
        language: source_doc.language,
    })
}

#[command]
pub async fn import_website(
    app: AppHandle,
    url: String,
    state: State<'_, AppState>,
) -> Result<ImportResult, ImportError> {
    // Validate URL format
    let parsed = url::Url::parse(&url).map_err(|_| ImportError {
        code: "INVALID_URL".to_string(),
        message: format!("Invalid URL: {}", url),
    })?;

    // Security: reject internal URLs
    let host = parsed.host_str().unwrap_or("");
    if is_private_host(host) {
        return Err(ImportError {
            code: "URL_BLOCKED".to_string(),
            message: "Cannot import from internal/private URLs".to_string(),
        });
    }

    // Emit progress
    let _ = app.emit(
        "import:progress",
        serde_json::json!({
            "stage": "fetching",
            "path": url
        }),
    );

    // Import via content pipeline
    let job_id = state.content_pipeline.import_website_job(&url).await;
    let source_doc = state.content_pipeline.import_website(&url).await?;
    if let Some(mut job) = state.content_pipeline.get_import_job(job_id).await {
        job.mark_completed(source_doc.id);
        state.content_pipeline.upsert_import_job(job).await;
    }

    // Emit completion
    let _ = app.emit(
        "import:complete",
        serde_json::json!({
            "doc_id": source_doc.id.to_string(),
            "title": source_doc.title,
            "chunks": source_doc.chunks.len(),
            "word_count": source_doc.metadata.word_count
        }),
    );

    Ok(ImportResult {
        doc_id: source_doc.id.to_string(),
        title: source_doc.title,
        source_type: source_doc.source_type.to_string(),
        checksum: source_doc.checksum,
        chunks: source_doc.chunks.len() as u32,
        word_count: source_doc.metadata.word_count,
        language: source_doc.language,
    })
}

fn is_private_host(host: &str) -> bool {
    host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("192.168.")
        || host.starts_with("10.")
        || host.starts_with("172.16.")
        || host.starts_with("172.17.")
        || host.starts_with("172.18.")
        || host.starts_with("172.19.")
        || host.starts_with("172.20.")
        || host.starts_with("172.21.")
        || host.starts_with("172.22.")
        || host.starts_with("172.23.")
        || host.starts_with("172.24.")
        || host.starts_with("172.25.")
        || host.starts_with("172.26.")
        || host.starts_with("172.27.")
        || host.starts_with("172.28.")
        || host.starts_with("172.29.")
        || host.starts_with("172.30.")
        || host.starts_with("172.31.")
        || host.starts_with("169.254.")
        || host.starts_with("0.")
}
