use chrono::{DateTime, Utc};
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

pub async fn list_source_documents(
    pool: &SqlitePool,
    session_id: Uuid,
) -> Result<Vec<serde_json::Value>, StorageError> {
    type SourceSummaryRow = (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        DateTime<Utc>,
        i64,
    );

    let rows: Vec<SourceSummaryRow> = sqlx::query_as(
        "SELECT d.id, d.source_type, d.title, d.origin, d.checksum, d.language, \
         d.metadata, d.extracted_at, COUNT(c.id) AS chunk_count \
         FROM source_documents d \
         LEFT JOIN source_chunks c ON c.document_id = d.id \
         WHERE d.session_id = ? \
         GROUP BY d.id \
         ORDER BY d.extracted_at DESC",
    )
    .bind(session_id.to_string())
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(
            |(
                id,
                source_type,
                title,
                origin,
                checksum,
                language,
                metadata,
                extracted_at,
                chunk_count,
            )| {
                let metadata_json: serde_json::Value = serde_json::from_str(&metadata)?;
                Ok(serde_json::json!({
                    "id": id,
                    "source_type": source_type,
                    "title": title,
                    "origin": origin,
                    "checksum": checksum,
                    "language": language,
                    "extracted_at": extracted_at,
                    "chunk_count": chunk_count,
                    "word_count": metadata_json
                        .get("word_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                }))
            },
        )
        .collect()
}

pub async fn get_source_document(
    pool: &SqlitePool,
    session_id: Uuid,
    document_id: Uuid,
) -> Result<Option<serde_json::Value>, StorageError> {
    type DocumentRow = (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
        DateTime<Utc>,
    );

    let row: Option<DocumentRow> = sqlx::query_as(
        "SELECT id, source_type, title, origin, checksum, language, \
         license_or_usage_note, metadata, extracted_at \
         FROM source_documents \
         WHERE id = ? AND session_id = ?",
    )
    .bind(document_id.to_string())
    .bind(session_id.to_string())
    .fetch_optional(pool)
    .await?;

    let Some((
        id,
        source_type,
        title,
        origin,
        checksum,
        language,
        license_or_usage_note,
        metadata,
        extracted_at,
    )) = row
    else {
        return Ok(None);
    };

    type ChunkRow = (String, String, i64, String, String, i64, bool);
    let chunk_rows: Vec<ChunkRow> = sqlx::query_as(
        "SELECT id, document_id, chunk_index, content, heading_path, token_count, \
         overlap_with_previous \
         FROM source_chunks \
         WHERE document_id = ? \
         ORDER BY chunk_index ASC",
    )
    .bind(&id)
    .fetch_all(pool)
    .await?;

    let chunks: Result<Vec<_>, StorageError> = chunk_rows
        .into_iter()
        .map(
            |(
                id,
                document_id,
                index,
                content,
                heading_path,
                token_count,
                overlap_with_previous,
            )| {
                let heading_path: Vec<String> = serde_json::from_str(&heading_path)?;
                Ok(serde_json::json!({
                    "id": id,
                    "document_id": document_id,
                    "index": index,
                    "content": content,
                    "heading_path": heading_path,
                    "token_count": token_count,
                    "overlap_with_previous": overlap_with_previous,
                }))
            },
        )
        .collect();

    Ok(Some(serde_json::json!({
        "id": id,
        "source_type": source_type,
        "title": title,
        "origin": origin,
        "checksum": checksum,
        "language": language,
        "license_or_usage_note": license_or_usage_note,
        "extracted_at": extracted_at,
        "metadata": serde_json::from_str::<serde_json::Value>(&metadata)?,
        "chunks": chunks?,
    })))
}

pub async fn get_import_job(
    pool: &SqlitePool,
    session_id: Uuid,
    job_id: Uuid,
) -> Result<Option<serde_json::Value>, StorageError> {
    type ImportJobRow = (
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        String,
        String,
        Option<String>,
        Option<String>,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
    );

    let row: Option<ImportJobRow> = sqlx::query_as(
        "SELECT id, session_id, source_type, source_path, source_url, config, \
         status, error, result_document_id, created_at, completed_at \
         FROM import_jobs \
         WHERE id = ? AND session_id = ?",
    )
    .bind(job_id.to_string())
    .bind(session_id.to_string())
    .fetch_optional(pool)
    .await?;

    row.map(
        |(
            id,
            session_id,
            source_type,
            source_path,
            source_url,
            config,
            status,
            error,
            result_document_id,
            created_at,
            completed_at,
        )| {
            let config: serde_json::Value = serde_json::from_str(&config)?;
            let error = match error {
                Some(error) => Some(serde_json::from_str::<serde_json::Value>(&error)?),
                None => None,
            };

            Ok(serde_json::json!({
                "id": id,
                "session_id": session_id,
                "source_type": source_type,
                "source_path": source_path,
                "source_url": source_url,
                "config": config,
                "status": status,
                "error": error,
                "result_document_id": result_document_id,
                "created_at": created_at,
                "completed_at": completed_at,
            }))
        },
    )
    .transpose()
}
