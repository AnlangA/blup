use axum::{extract::State, Json};
use uuid::Uuid;

use assessment_engine::models::exercise::{Difficulty, Exercise, ExerciseType, RubricDimension};

use super::helpers::load_or_404;
use crate::error::ApiError;
use crate::AppState;

#[derive(serde::Deserialize)]
pub struct ExerciseSubmission {
    pub answer: serde_json::Value,
}

/// Build an assessment-engine Exercise from the curriculum-level exercise data.
/// The curriculum only stores question + type string, so we construct reasonable
/// defaults for each type.
fn build_assessment_exercise(ch_id: &str, question: &str, exercise_type: &str) -> Exercise {
    let max_score = 1.0;
    match exercise_type {
        "multiple_choice" => Exercise::new_multiple_choice(
            ch_id,
            question,
            vec![
                "Option A".to_string(),
                "Option B".to_string(),
                "Option C".to_string(),
                "Option D".to_string(),
            ],
            0,
            max_score,
        ),
        "short_answer" => Exercise::new_short_answer(ch_id, question, "", vec![], max_score),
        "coding" => Exercise::new_coding(ch_id, question, "python", vec![], max_score),
        "reflection" => Exercise {
            id: Uuid::new_v4(),
            chapter_id: ch_id.to_string(),
            question: question.to_string(),
            exercise_type: ExerciseType::Reflection {
                prompt: question.to_string(),
                min_length: 50,
                rubric_dimensions: vec![RubricDimension {
                    name: "insight".to_string(),
                    description: "Demonstrates understanding of the topic".to_string(),
                    max_score,
                }],
            },
            difficulty: Difficulty::Medium,
            rubric: None,
            max_score,
            hints: vec![],
            explanation: None,
        },
        _ => {
            tracing::warn!(
                exercise_type = exercise_type,
                "Unknown exercise type, defaulting to reflection"
            );
            Exercise {
                id: Uuid::new_v4(),
                chapter_id: ch_id.to_string(),
                question: question.to_string(),
                exercise_type: ExerciseType::Reflection {
                    prompt: question.to_string(),
                    min_length: 50,
                    rubric_dimensions: vec![],
                },
                difficulty: Difficulty::Medium,
                rubric: None,
                max_score,
                hints: vec![],
                explanation: None,
            }
        }
    }
}

pub async fn submit_exercise(
    State(state): State<AppState>,
    axum::extract::Path((id, ch_id, ex_id)): axum::extract::Path<(Uuid, String, String)>,
    Json(submission): Json<ExerciseSubmission>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let handle = load_or_404(&state, id).await?;
    let s = handle.read().await;

    let curriculum = s
        .curriculum
        .as_ref()
        .ok_or_else(|| ApiError::Validation("No curriculum available".to_string()))?;

    let chapter = curriculum
        .chapters
        .iter()
        .find(|c| c.id == ch_id)
        .ok_or(ApiError::NotFound)?;

    let exercise_data = chapter
        .exercises
        .iter()
        .find(|e| e.question == ex_id || e.question.contains(&ex_id))
        .or_else(|| chapter.exercises.first())
        .ok_or_else(|| {
            ApiError::Validation("No exercises available for this chapter".to_string())
        })?;

    let assessment_exercise = build_assessment_exercise(
        &ch_id,
        &exercise_data.question,
        &exercise_data.exercise_type,
    );

    let evaluation = state
        .assessment
        .evaluate(&assessment_exercise, &submission.answer)
        .map_err(|e| ApiError::Internal(format!("Assessment evaluation failed: {e}")))?;

    let evaluation_value = serde_json::to_value(&evaluation)
        .map_err(|e| ApiError::Internal(format!("Evaluation serialize: {e}")))?;

    let exercise_value =
        serde_json::to_value(&assessment_exercise).unwrap_or_else(|_| serde_json::json!({}));

    if let Err(e) = state
        .storage
        .save_assessment(
            id,
            Some(&ch_id),
            &storage::models::assessment::AssessmentInput {
                exercise: exercise_value,
                answer: Some(submission.answer),
                evaluation: Some(evaluation_value.clone()),
                score: Some(evaluation.score),
                max_score: Some(evaluation.max_score),
            },
        )
        .await
    {
        tracing::warn!(session_id = %id, chapter_id = %ch_id, error = %e, "Failed to persist assessment to storage");
    }

    Ok(Json(evaluation_value))
}
