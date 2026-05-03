use sqlx::SqlitePool;
use uuid::Uuid;

use serde::Serialize;

use crate::error::StorageError;
#[derive(Debug, Clone, Serialize)]
pub struct StoredSourceDocument<'a> {
    pub id: Uuid,
    pub source_type: &'a str,
    pub title: &'a str,
    pub origin: &'a str,
    pub checksum: &'a str,
    pub language: Option<&'a str>,
    pub license_or_usage_note: Option<&'a str>,
    pub metadata: &'a serde_json::Value,
    pub extracted_at: chrono::DateTime<chrono::Utc>,
    pub chunks: &'a [StoredSourceChunk<'a>],
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredSourceChunk<'a> {
    pub id: Uuid,
    pub document_id: Uuid,
    pub index: u32,
    pub content: &'a str,
    pub heading_path: &'a [String],
    pub token_count: u32,
    pub overlap_with_previous: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredImportJob<'a> {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub source_type: &'a str,
    pub source_path: Option<&'a str>,
    pub source_url: Option<&'a str>,
    pub config: &'a serde_json::Value,
    pub status: &'a str,
    pub error: Option<&'a serde_json::Value>,
    pub result_document_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoredExportJob<'a> {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub export_type: &'a str,
    pub source_id: &'a str,
    pub config: &'a serde_json::Value,
    pub status: &'a str,
    pub error: Option<&'a serde_json::Value>,
    pub result_artifact_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn save_source_document(
    pool: &SqlitePool,
    session_id: Option<Uuid>,
    document: &StoredSourceDocument<'_>,
) -> Result<(), StorageError> {
    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT OR REPLACE INTO source_documents \
         (id, session_id, source_type, title, origin, checksum, language, license_or_usage_note, metadata, extracted_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(document.id.to_string())
    .bind(session_id.map(|id| id.to_string()))
    .bind(document.source_type)
    .bind(document.title)
    .bind(document.origin)
    .bind(document.checksum)
    .bind(document.language)
    .bind(document.license_or_usage_note)
    .bind(serde_json::to_string(document.metadata)?)
    .bind(document.extracted_at)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM source_chunks WHERE document_id = ?")
        .bind(document.id.to_string())
        .execute(&mut *tx)
        .await?;

    for chunk in document.chunks {
        sqlx::query(
            "INSERT INTO source_chunks \
             (id, document_id, chunk_index, content, heading_path, token_count, overlap_with_previous) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(chunk.id.to_string())
        .bind(chunk.document_id.to_string())
        .bind(chunk.index as i64)
        .bind(chunk.content)
        .bind(serde_json::to_string(chunk.heading_path)?)
        .bind(chunk.token_count as i64)
        .bind(chunk.overlap_with_previous)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn save_import_job(
    pool: &SqlitePool,
    job: &StoredImportJob<'_>,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT OR REPLACE INTO import_jobs \
         (id, session_id, source_type, source_path, source_url, config, status, error, result_document_id, created_at, completed_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(job.id.to_string())
    .bind(job.session_id.map(|id| id.to_string()))
    .bind(job.source_type)
    .bind(job.source_path)
    .bind(job.source_url)
    .bind(serde_json::to_string(job.config)?)
    .bind(job.status)
    .bind(match job.error {
        Some(error) => Some(serde_json::to_string(error)?),
        None => None,
    })
    .bind(job.result_document_id.map(|id| id.to_string()))
    .bind(job.created_at)
    .bind(job.completed_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn save_export_job(
    pool: &SqlitePool,
    job: &StoredExportJob<'_>,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT OR REPLACE INTO export_jobs \
         (id, session_id, export_type, source_id, config, status, error, result_artifact_id, created_at, completed_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(job.id.to_string())
    .bind(job.session_id.map(|id| id.to_string()))
    .bind(job.export_type)
    .bind(job.source_id)
    .bind(serde_json::to_string(job.config)?)
    .bind(job.status)
    .bind(match job.error {
        Some(error) => Some(serde_json::to_string(error)?),
        None => None,
    })
    .bind(job.result_artifact_id.map(|id| id.to_string()))
    .bind(job.created_at)
    .bind(job.completed_at)
    .execute(pool)
    .await?;

    Ok(())
}
