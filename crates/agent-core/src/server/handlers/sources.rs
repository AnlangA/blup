use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use content_pipeline::error::ImportError as PipelineImportError;
use content_pipeline::models::{ImportJob, ImportStatus, SourceDocument};

use super::helpers::load_or_404;
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct WebsiteImportRequest {
    pub url: String,
}

pub async fn list_sources(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = load_or_404(&state, id).await?;
    let sources = state.storage.list_source_documents(id).await?;
    Ok(Json(json!({ "sources": sources })))
}

pub async fn get_source(
    State(state): State<AppState>,
    axum::extract::Path((id, source_id)): axum::extract::Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = load_or_404(&state, id).await?;
    let source = state
        .storage
        .get_source_document(id, source_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(source))
}

pub async fn get_import_job(
    State(state): State<AppState>,
    axum::extract::Path((id, job_id)): axum::extract::Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = load_or_404(&state, id).await?;
    let job = state
        .storage
        .get_import_job(id, job_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(job))
}

pub async fn import_website(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
    Json(req): Json<WebsiteImportRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let _ = load_or_404(&state, id).await?;

    let mut job = ImportJob::new_website(&req.url);
    job.session_id = Some(id);
    job.status = ImportStatus::Extracting;
    save_import_job(&state, &job).await?;

    match state.content_pipeline.import_website(&req.url).await {
        Ok(document) => {
            save_source_document(&state, id, &document).await?;
            job.mark_completed(document.id);
            save_import_job(&state, &job).await?;
            Ok(Json(json!({
                "job_id": job.id,
                "document": document,
                "job": job,
            })))
        }
        Err(err) => {
            job.mark_failed(import_error_code(&err), &safe_import_error_message(&err));
            save_import_job(&state, &job).await?;
            Err(map_import_error(err))
        }
    }
}

fn import_error_code(err: &PipelineImportError) -> &'static str {
    match err {
        PipelineImportError::InvalidUrl(_) => "INVALID_URL",
        PipelineImportError::UrlBlocked { .. } => "URL_BLOCKED",
        PipelineImportError::ContentTooShort { .. } => "CONTENT_TOO_SHORT",
        PipelineImportError::NoContent(_) => "NO_CONTENT",
        PipelineImportError::FetchFailed { .. } | PipelineImportError::Http(_) => "FETCH_FAILED",
        _ => "IMPORT_FAILED",
    }
}

fn safe_import_error_message(err: &PipelineImportError) -> String {
    match err {
        PipelineImportError::InvalidUrl(_) => "Invalid URL".to_string(),
        PipelineImportError::UrlBlocked { reason, .. } => reason.clone(),
        PipelineImportError::FetchFailed { reason, .. } => {
            format!("Website fetch failed: {reason}")
        }
        PipelineImportError::ContentTooShort { length, .. } => {
            format!("Imported content is too short ({length} chars)")
        }
        PipelineImportError::NoContent(_) => "No extractable content found".to_string(),
        other => other.to_string(),
    }
}

fn map_import_error(err: PipelineImportError) -> ApiError {
    let message = safe_import_error_message(&err);
    match err {
        PipelineImportError::InvalidUrl(_)
        | PipelineImportError::UrlBlocked { .. }
        | PipelineImportError::ContentTooShort { .. }
        | PipelineImportError::NoContent(_) => ApiError::Validation(message),
        _ => ApiError::Internal(message),
    }
}

async fn save_import_job(state: &AppState, job: &ImportJob) -> Result<(), ApiError> {
    let config = serde_json::to_value(&job.config)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize import config: {e}")))?;
    let error = job
        .error
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ApiError::Internal(format!("Failed to serialize import error: {e}")))?;
    let status = serde_json::to_value(&job.status)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "pending".to_string());
    let source_type = job.source_type.to_string();

    let stored = storage::models::content::StoredImportJob {
        id: job.id,
        session_id: job.session_id,
        source_type: &source_type,
        source_path: job.source_path.as_deref(),
        source_url: job.source_url.as_deref(),
        config: &config,
        status: &status,
        error: error.as_ref(),
        result_document_id: job.result_document_id,
        created_at: job.created_at,
        completed_at: job.completed_at,
    };
    state.storage.save_import_job(&stored).await?;
    Ok(())
}

async fn save_source_document(
    state: &AppState,
    session_id: Uuid,
    document: &SourceDocument,
) -> Result<(), ApiError> {
    let metadata = serde_json::to_value(&document.metadata)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize source metadata: {e}")))?;
    let chunks = document
        .chunks
        .iter()
        .map(|chunk| storage::models::content::StoredSourceChunk {
            id: chunk.id,
            document_id: chunk.document_id,
            index: chunk.index,
            content: &chunk.content,
            heading_path: &chunk.heading_path,
            token_count: chunk.token_count,
            overlap_with_previous: chunk.overlap_with_previous,
        })
        .collect::<Vec<_>>();
    let source_type = document.source_type.to_string();
    let stored = storage::models::content::StoredSourceDocument {
        id: document.id,
        source_type: &source_type,
        title: &document.title,
        origin: &document.origin,
        checksum: &document.checksum,
        language: document.language.as_deref(),
        license_or_usage_note: document.license_or_usage_note.as_deref(),
        metadata: &metadata,
        extracted_at: document.extracted_at,
        chunks: &chunks,
    };
    state
        .storage
        .save_source_document(Some(session_id), &stored)
        .await?;
    Ok(())
}
