use sqlx::SqlitePool;
use uuid::Uuid;

use crate::error::StorageError;

#[derive(serde::Serialize)]
pub struct AssessmentInput {
    pub exercise: serde_json::Value,
    pub answer: Option<serde_json::Value>,
    pub evaluation: Option<serde_json::Value>,
    pub score: Option<f64>,
    pub max_score: Option<f64>,
}

pub async fn save_assessment(
    pool: &SqlitePool,
    session_id: Uuid,
    chapter_id: Option<&str>,
    input: &AssessmentInput,
) -> Result<(), StorageError> {
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now();
    let evaluated_at = if input.evaluation.is_some() {
        Some(now)
    } else {
        None
    };

    sqlx::query(
        "INSERT INTO assessments (id, session_id, chapter_id, exercise, learner_answer, evaluation, score, max_score, created_at, evaluated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(session_id.to_string())
    .bind(chapter_id)
    .bind(serde_json::to_string(&input.exercise)?)
    .bind(input.answer.as_ref().map(serde_json::to_string).transpose()?)
    .bind(input.evaluation.as_ref().map(serde_json::to_string).transpose()?)
    .bind(input.score)
    .bind(input.max_score)
    .bind(now)
    .bind(evaluated_at)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_assessments(
    pool: &SqlitePool,
    session_id: Uuid,
) -> Result<Vec<serde_json::Value>, StorageError> {
    type AssessmentRow = (
        String,
        String,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<f64>,
        Option<f64>,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    );

    let rows: Vec<AssessmentRow> = sqlx::query_as(
        "SELECT id, session_id, chapter_id, exercise, learner_answer, evaluation, score, max_score, created_at, evaluated_at FROM assessments WHERE session_id = ? ORDER BY created_at"
    )
    .bind(session_id.to_string())
    .fetch_all(pool)
    .await?;

    let mut assessments = Vec::new();
    for (
        id,
        session_id,
        chapter_id,
        exercise,
        learner_answer,
        evaluation,
        score,
        max_score,
        created_at,
        evaluated_at,
    ) in rows
    {
        let exercise_val: serde_json::Value = serde_json::from_str(&exercise)?;
        let answer_val: Option<serde_json::Value> = learner_answer
            .map(|a| serde_json::from_str(&a))
            .transpose()?;
        let evaluation_val: Option<serde_json::Value> =
            evaluation.map(|e| serde_json::from_str(&e)).transpose()?;

        let assessment = serde_json::json!({
            "id": id,
            "session_id": session_id,
            "chapter_id": chapter_id,
            "exercise": exercise_val,
            "learner_answer": answer_val,
            "evaluation": evaluation_val,
            "score": score,
            "max_score": max_score,
            "created_at": created_at,
            "evaluated_at": evaluated_at
        });
        assessments.push(assessment);
    }

    Ok(assessments)
}
