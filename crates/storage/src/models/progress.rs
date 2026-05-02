use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::StorageError;

type ProgressRow = (
    String,
    String,
    String,
    String,
    f64,
    i64,
    i64,
    i64,
    Option<i64>,
);

pub async fn upsert_progress(
    pool: &SqlitePool,
    session_id: Uuid,
    chapter_id: &str,
    progress: serde_json::Value,
) -> Result<(), StorageError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();

    // Try to update existing progress first
    let result = sqlx::query(
        "UPDATE chapter_progress SET status = ?, completion = ?, time_spent_minutes = ?, exercises_completed = ?, exercises_total = ?, difficulty_rating = ?, last_accessed = ?, updated_at = ? WHERE session_id = ? AND chapter_id = ?"
    )
    .bind(progress.get("status").and_then(|v| v.as_str()).unwrap_or("not_started"))
    .bind(progress.get("completion").and_then(|v| v.as_f64()).unwrap_or(0.0))
    .bind(progress.get("time_spent_minutes").and_then(|v| v.as_i64()).unwrap_or(0))
    .bind(progress.get("exercises_completed").and_then(|v| v.as_i64()).unwrap_or(0))
    .bind(progress.get("exercises_total").and_then(|v| v.as_i64()).unwrap_or(0))
    .bind(progress.get("difficulty_rating").and_then(|v| v.as_i64()))
    .bind(now)
    .bind(now)
    .bind(session_id.to_string())
    .bind(chapter_id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        // Insert new progress
        sqlx::query(
            "INSERT INTO chapter_progress (id, session_id, chapter_id, status, completion, time_spent_minutes, exercises_completed, exercises_total, difficulty_rating, last_accessed, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(session_id.to_string())
        .bind(chapter_id)
        .bind(progress.get("status").and_then(|v| v.as_str()).unwrap_or("not_started"))
        .bind(progress.get("completion").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .bind(progress.get("time_spent_minutes").and_then(|v| v.as_i64()).unwrap_or(0))
        .bind(progress.get("exercises_completed").and_then(|v| v.as_i64()).unwrap_or(0))
        .bind(progress.get("exercises_total").and_then(|v| v.as_i64()).unwrap_or(0))
        .bind(progress.get("difficulty_rating").and_then(|v| v.as_i64()))
        .bind(now)
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn get_progress(
    pool: &SqlitePool,
    session_id: Uuid,
    chapter_id: &str,
) -> Result<Option<serde_json::Value>, StorageError> {
    let row: Option<ProgressRow> = sqlx::query_as(
        "SELECT id, session_id, chapter_id, status, completion, time_spent_minutes, exercises_completed, exercises_total, difficulty_rating FROM chapter_progress WHERE session_id = ? AND chapter_id = ?"
    )
    .bind(session_id.to_string())
    .bind(chapter_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((
            id,
            session_id,
            chapter_id,
            status,
            completion,
            time_spent,
            exercises_completed,
            exercises_total,
            difficulty_rating,
        )) => {
            let progress = serde_json::json!({
                "id": id,
                "session_id": session_id,
                "chapter_id": chapter_id,
                "status": status,
                "completion": completion,
                "time_spent_minutes": time_spent,
                "exercises_completed": exercises_completed,
                "exercises_total": exercises_total,
                "difficulty_rating": difficulty_rating
            });
            Ok(Some(progress))
        }
        None => Ok(None),
    }
}

pub async fn get_all_progress(
    pool: &SqlitePool,
    session_id: Uuid,
) -> Result<Vec<serde_json::Value>, StorageError> {
    let rows: Vec<ProgressRow> = sqlx::query_as(
        "SELECT id, session_id, chapter_id, status, completion, time_spent_minutes, exercises_completed, exercises_total, difficulty_rating FROM chapter_progress WHERE session_id = ? ORDER BY created_at"
    )
    .bind(session_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut progress_list = Vec::new();
    for (
        id,
        session_id,
        chapter_id,
        status,
        completion,
        time_spent,
        exercises_completed,
        exercises_total,
        difficulty_rating,
    ) in rows
    {
        let progress = serde_json::json!({
            "id": id,
            "session_id": session_id,
            "chapter_id": chapter_id,
            "status": status,
            "completion": completion,
            "time_spent_minutes": time_spent,
            "exercises_completed": exercises_completed,
            "exercises_total": exercises_total,
            "difficulty_rating": difficulty_rating
        });
        progress_list.push(progress);
    }

    Ok(progress_list)
}
