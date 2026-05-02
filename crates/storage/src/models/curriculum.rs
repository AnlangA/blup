use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::StorageError;

pub async fn save_curriculum(
    pool: &SqlitePool,
    session_id: Uuid,
    curriculum: serde_json::Value,
) -> Result<(), StorageError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Store the full curriculum JSON for complete round-trip fidelity.
    // Also extract top-level fields for query convenience.
    let full_json = serde_json::to_string(&curriculum)?;

    // Use a transaction for atomicity (delete old + insert new)
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM curricula WHERE session_id = ?")
        .bind(session_id.to_string())
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO curricula (id, session_id, title, description, estimated_duration, learning_objectives, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(session_id.to_string())
    .bind(curriculum.get("title").and_then(|v| v.as_str()).unwrap_or(""))
    .bind(curriculum.get("description").and_then(|v| v.as_str()).unwrap_or(""))
    .bind(curriculum.get("estimated_duration").and_then(|v| v.as_str()).unwrap_or(""))
    .bind(&full_json)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn get_curriculum(
    pool: &SqlitePool,
    session_id: Uuid,
) -> Result<Option<serde_json::Value>, StorageError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT learning_objectives FROM curricula WHERE session_id = ?")
            .bind(session_id.to_string())
            .fetch_optional(pool)
            .await?;

    match row {
        Some((json_str,)) => {
            // The learning_objectives column stores the full curriculum JSON
            let curriculum: serde_json::Value = serde_json::from_str(&json_str)?;
            Ok(Some(curriculum))
        }
        None => Ok(None),
    }
}
