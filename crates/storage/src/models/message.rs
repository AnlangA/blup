use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::StorageError;

pub async fn save_message(
    pool: &SqlitePool,
    session_id: Uuid,
    chapter_id: Option<&str>,
    role: &str,
    content: &str,
) -> Result<(), StorageError> {
    let id = Uuid::new_v4().to_string();
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO messages (id, session_id, chapter_id, role, content, created_at) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(session_id.to_string())
    .bind(chapter_id)
    .bind(role)
    .bind(content)
    .bind(now)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_messages(
    pool: &SqlitePool,
    session_id: Uuid,
    limit: i64,
    before: Option<DateTime<Utc>>,
) -> Result<Vec<serde_json::Value>, StorageError> {
    type MessageRow = (
        String,
        String,
        Option<String>,
        String,
        String,
        DateTime<Utc>,
    );

    let rows: Vec<MessageRow> = if let Some(before_time) = before {
        sqlx::query_as(
            "SELECT id, session_id, chapter_id, role, content, created_at FROM messages WHERE session_id = ? AND created_at < ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(session_id.to_string())
        .bind(before_time)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as(
            "SELECT id, session_id, chapter_id, role, content, created_at FROM messages WHERE session_id = ? ORDER BY created_at DESC LIMIT ?"
        )
        .bind(session_id.to_string())
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let mut messages = Vec::new();
    for (id, session_id, chapter_id, role, content, created_at) in rows {
        let message = serde_json::json!({
            "id": id,
            "session_id": session_id,
            "chapter_id": chapter_id,
            "role": role,
            "content": content,
            "created_at": created_at
        });
        messages.push(message);
    }

    Ok(messages)
}
