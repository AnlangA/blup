use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::Json;
use serde_json::json;
use uuid::Uuid;

use content_pipeline::error::ExportError as PipelineExportError;
use content_pipeline::export::TypstCompiler;
use content_pipeline::models::{DocumentArtifact, ExportJob, ExportStatus};

use super::helpers::{load_or_404, next_sse_id, sse_serialize};
use super::types::SseEvent;
use crate::error::ApiError;
use crate::AppState;

fn render_error_code(err: &PipelineExportError) -> &'static str {
    match err {
        PipelineExportError::InvalidMarkdown(_) => "VALIDATION_ERROR",
        _ => "RENDER_ERROR",
    }
}

fn map_render_error(err: PipelineExportError, action: &str) -> ApiError {
    match err {
        PipelineExportError::InvalidMarkdown(message) => ApiError::Validation(message),
        other => ApiError::Internal(format!("Failed to {action}: {other}")),
    }
}

fn build_chapter_json(
    content: String,
    chapter_meta: Option<&crate::state::domain::ChapterData>,
    fallback_title: &str,
) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    map.insert("content".to_string(), serde_json::Value::String(content));

    if let Some(ch) = chapter_meta {
        map.insert(
            "title".to_string(),
            serde_json::Value::String(ch.title.clone()),
        );
        if let Some(minutes) = ch.estimated_minutes {
            map.insert(
                "estimated_minutes".to_string(),
                serde_json::Value::Number(minutes.into()),
            );
        }
        if !ch.objectives.is_empty() {
            map.insert(
                "objectives".to_string(),
                serde_json::Value::Array(
                    ch.objectives
                        .iter()
                        .map(|o| serde_json::Value::String(o.clone()))
                        .collect(),
                ),
            );
        }
        if !ch.prerequisites.is_empty() {
            map.insert(
                "prerequisites".to_string(),
                serde_json::Value::Array(
                    ch.prerequisites
                        .iter()
                        .map(|p| serde_json::Value::String(p.clone()))
                        .collect(),
                ),
            );
        }
        if !ch.key_concepts.is_empty() {
            map.insert(
                "key_concepts".to_string(),
                serde_json::Value::Array(
                    ch.key_concepts
                        .iter()
                        .map(|k| serde_json::Value::String(k.clone()))
                        .collect(),
                ),
            );
        }
        if !ch.exercises.is_empty() {
            if let Ok(v) = serde_json::to_value(&ch.exercises) {
                map.insert("exercises".to_string(), v);
            }
        }
    } else {
        map.insert(
            "title".to_string(),
            serde_json::Value::String(fallback_title.to_string()),
        );
    }

    serde_json::Value::Object(map)
}

// ── export_chapter_typst (sync JSON) ──

pub async fn export_chapter_typst(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let mut job = ExportJob::new_chapter(&ch_id);
    job.session_id = Some(id);
    job.status = ExportStatus::Rendering;
    save_export_job(&state.storage, &job).await?;

    let chapter_json = {
        let s = handle.read().await;
        let content = s
            .chapter_contents
            .get(&ch_id)
            .cloned()
            .ok_or(ApiError::NotFound)?;
        let chapter_meta = s
            .curriculum
            .as_ref()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == ch_id));
        build_chapter_json(content, chapter_meta, &ch_id)
    };

    let chapter_title = chapter_json
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("chapter");

    let typst_source = match state
        .content_pipeline
        .render_chapter_to_typst(&chapter_json)
    {
        Ok(source) => source,
        Err(err) => {
            job.mark_failed(render_error_code(&err), &err.to_string(), None);
            save_export_job(&state.storage, &job).await?;
            return Err(map_render_error(err, "render chapter to Typst"));
        }
    };

    let artifact = DocumentArtifact::new_typst(&typst_source);
    job.mark_completed(artifact.id);
    save_export_job(&state.storage, &job).await?;

    let sanitized = sanitize_filename(chapter_title);
    let filename = format!("{}.typ", sanitized);

    Ok(Json(json!({
        "job_id": job.id,
        "filename": filename,
        "typst_source": typst_source,
        "checksum": artifact.checksum,
        "size_bytes": artifact.size_bytes,
        "compiled": false,
    })))
}

pub async fn export_curriculum_typst(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let mut job = ExportJob::new_curriculum("curriculum");
    job.session_id = Some(id);
    job.status = ExportStatus::Rendering;
    save_export_job(&state.storage, &job).await?;

    let curriculum_json = {
        let s = handle.read().await;
        let curriculum = s.curriculum.as_ref().ok_or_else(|| {
            ApiError::InvalidTransition("No curriculum available to export".to_string())
        })?;
        serde_json::to_value(curriculum)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize curriculum: {e}")))?
    };

    let typst_source = match state
        .content_pipeline
        .render_curriculum_to_typst(&curriculum_json)
    {
        Ok(source) => source,
        Err(err) => {
            job.mark_failed(render_error_code(&err), &err.to_string(), None);
            save_export_job(&state.storage, &job).await?;
            return Err(map_render_error(err, "render curriculum to Typst"));
        }
    };

    let artifact = DocumentArtifact::new_typst(&typst_source);
    job.mark_completed(artifact.id);
    save_export_job(&state.storage, &job).await?;

    let title = curriculum_json
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("curriculum");
    let sanitized = sanitize_filename(title);
    let filename = format!("{}.typ", sanitized);

    Ok(Json(json!({
        "job_id": job.id,
        "filename": filename,
        "typst_source": typst_source,
        "checksum": artifact.checksum,
        "size_bytes": artifact.size_bytes,
        "compiled": false,
    })))
}

// ── export_chapter_pdf_stream (SSE) ──

pub async fn export_chapter_pdf_stream(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id)): axum::extract::Path<(Uuid, String)>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let mut job = ExportJob::new_chapter(&ch_id);
    job.session_id = Some(id);
    job.status = ExportStatus::Rendering;
    save_export_job(&state.storage, &job).await?;

    let (chapter_json, chapter_title) = {
        let s = handle.read().await;
        let content = s
            .chapter_contents
            .get(&ch_id)
            .cloned()
            .ok_or(ApiError::NotFound)?;
        let chapter_meta = s
            .curriculum
            .as_ref()
            .and_then(|c| c.chapters.iter().find(|ch| ch.id == ch_id));
        let title = chapter_meta
            .map(|ch| ch.title.clone())
            .unwrap_or_else(|| ch_id.clone());
        let json = build_chapter_json(content, chapter_meta, &ch_id);
        (json, title)
    };

    let pipeline = state.content_pipeline.clone();
    let sandbox = state.sandbox_manager.clone();
    let storage = state.storage.clone();
    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);
    let sanitized = sanitize_filename(&chapter_title);

    let stream = async_stream::stream! {
        let mut job = job;
        yield Ok(Event::default()
            .event("status")
            .id(next_sse_id())
            .data(sse_serialize(&SseEvent::Status {
                state: "rendering".to_string(),
                message: "Rendering chapter to Typst...".to_string(),
            })));

        let typst_source = match pipeline.render_chapter_to_typst(&chapter_json) {
            Ok(src) => src,
            Err(e) => {
                let code = render_error_code(&e).to_string();
                job.mark_failed(&code, &e.to_string(), None);
                let _ = save_export_job(&storage, &job).await;
                yield Ok(Event::default()
                    .event("error")
                    .id(next_sse_id())
                    .data(sse_serialize(&SseEvent::Error {
                        code,
                        message: format!("Failed to render: {e}"),
                    })));
                return;
            }
        };

        job.status = ExportStatus::Compiling;
        let _ = save_export_job(&storage, &job).await;

        yield Ok(Event::default()
            .event("status")
            .id(next_sse_id())
            .data(sse_serialize(&SseEvent::Status {
                state: "compiling".to_string(),
                message: "Compiling Typst to PDF...".to_string(),
            })));

        let compiler = TypstCompiler::new(sandbox.clone());
        match compiler.compile_to_pdf(&typst_source, &std::collections::HashMap::new()).await {
            Ok(artifact) => {
                job.mark_completed(artifact.id);
                let _ = save_export_job(&storage, &job).await;
                let pdf_base64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &artifact.data,
                );
                let filename = format!("{}.pdf", sanitized);

                yield Ok(Event::default()
                    .event("done")
                    .id(next_sse_id())
                    .data(sse_serialize(&SseEvent::Done {
                        result: json!({
                            "job_id": job.id,
                            "filename": filename,
                            "pdf_base64": pdf_base64,
                            "checksum": artifact.checksum,
                            "size_bytes": artifact.size_bytes,
                            "page_count": artifact.page_count,
                            "compiled": true,
                        }),
                    })));
            }
            Err(e) => {
                let diagnostics = export_diagnostics(&e);
                job.mark_failed("COMPILE_ERROR", &e.to_string(), diagnostics);
                let _ = save_export_job(&storage, &job).await;
                yield Ok(Event::default()
                    .event("error")
                    .id(next_sse_id())
                    .data(sse_serialize(&SseEvent::Error {
                        code: "COMPILE_ERROR".to_string(),
                        message: e.to_string(),
                    })));
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

// ── export_curriculum_pdf_stream (SSE) ──

pub async fn export_curriculum_pdf_stream(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, axum::Error>>>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let mut job = ExportJob::new_curriculum("curriculum");
    job.session_id = Some(id);
    job.status = ExportStatus::Rendering;
    save_export_job(&state.storage, &job).await?;

    let curriculum_json = {
        let s = handle.read().await;
        let curriculum = s.curriculum.as_ref().ok_or_else(|| {
            ApiError::InvalidTransition("No curriculum available to export".to_string())
        })?;
        serde_json::to_value(curriculum)
            .map_err(|e| ApiError::Internal(format!("Failed to serialize curriculum: {e}")))?
    };

    let title = curriculum_json
        .get("title")
        .and_then(|t| t.as_str())
        .unwrap_or("curriculum")
        .to_string();

    let pipeline = state.content_pipeline.clone();
    let sandbox = state.sandbox_manager.clone();
    let storage = state.storage.clone();
    let ping_interval = std::time::Duration::from_secs(state.config.sse_ping_interval_secs);
    let sanitized = sanitize_filename(&title);

    let stream = async_stream::stream! {
        let mut job = job;
        yield Ok(Event::default()
            .event("status")
            .id(next_sse_id())
            .data(sse_serialize(&SseEvent::Status {
                state: "rendering".to_string(),
                message: "Rendering curriculum to Typst...".to_string(),
            })));

        let typst_source = match pipeline.render_curriculum_to_typst(&curriculum_json) {
            Ok(src) => src,
            Err(e) => {
                let code = render_error_code(&e).to_string();
                job.mark_failed(&code, &e.to_string(), None);
                let _ = save_export_job(&storage, &job).await;
                yield Ok(Event::default()
                    .event("error")
                    .id(next_sse_id())
                    .data(sse_serialize(&SseEvent::Error {
                        code,
                        message: format!("Failed to render: {e}"),
                    })));
                return;
            }
        };

        job.status = ExportStatus::Compiling;
        let _ = save_export_job(&storage, &job).await;

        yield Ok(Event::default()
            .event("status")
            .id(next_sse_id())
            .data(sse_serialize(&SseEvent::Status {
                state: "compiling".to_string(),
                message: "Compiling Typst to PDF...".to_string(),
            })));

        let compiler = TypstCompiler::new(sandbox.clone());
        match compiler.compile_to_pdf(&typst_source, &std::collections::HashMap::new()).await {
            Ok(artifact) => {
                job.mark_completed(artifact.id);
                let _ = save_export_job(&storage, &job).await;
                let pdf_base64 = base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &artifact.data,
                );
                let filename = format!("{}.pdf", sanitized);

                yield Ok(Event::default()
                    .event("done")
                    .id(next_sse_id())
                    .data(sse_serialize(&SseEvent::Done {
                        result: json!({
                            "job_id": job.id,
                            "filename": filename,
                            "pdf_base64": pdf_base64,
                            "checksum": artifact.checksum,
                            "size_bytes": artifact.size_bytes,
                            "page_count": artifact.page_count,
                            "compiled": true,
                        }),
                    })));
            }
            Err(e) => {
                let diagnostics = export_diagnostics(&e);
                job.mark_failed("COMPILE_ERROR", &e.to_string(), diagnostics);
                let _ = save_export_job(&storage, &job).await;
                yield Ok(Event::default()
                    .event("error")
                    .id(next_sse_id())
                    .data(sse_serialize(&SseEvent::Error {
                        code: "COMPILE_ERROR".to_string(),
                        message: e.to_string(),
                    })));
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(ping_interval)))
}

// ── Helpers ──

fn export_diagnostics(
    err: &PipelineExportError,
) -> Option<Vec<content_pipeline::error::TypstDiagnostic>> {
    match err {
        PipelineExportError::CompilationFailed { diagnostics, .. } => Some(diagnostics.clone()),
        _ => None,
    }
}

async fn save_export_job(storage: &storage::Storage, job: &ExportJob) -> Result<(), ApiError> {
    let config = serde_json::to_value(&job.config)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize export config: {e}")))?;
    let error = job
        .error
        .as_ref()
        .map(serde_json::to_value)
        .transpose()
        .map_err(|e| ApiError::Internal(format!("Failed to serialize export error: {e}")))?;
    let export_type = serde_json::to_value(&job.export_type)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "chapter".to_string());
    let status = serde_json::to_value(&job.status)
        .ok()
        .and_then(|v| v.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "pending".to_string());

    let stored = storage::models::content::StoredExportJob {
        id: job.id,
        session_id: job.session_id,
        export_type: &export_type,
        source_id: &job.source_id,
        config: &config,
        status: &status,
        error: error.as_ref(),
        result_artifact_id: job.result_artifact_id,
        created_at: job.created_at,
        completed_at: job.completed_at,
    };
    storage.save_export_job(&stored).await?;
    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .replace(' ', "_")
        .trim_matches('_')
        .to_string()
}
