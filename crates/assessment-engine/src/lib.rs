pub mod error;
pub mod evaluation;
pub mod generation;
pub mod models;

use error::AssessmentError;
use models::evaluation::Evaluation;
use models::exercise::Exercise;

pub struct AssessmentEngine;

impl Clone for AssessmentEngine {
    fn clone(&self) -> Self {
        Self
    }
}

impl AssessmentEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn evaluate(
        &self,
        exercise: &Exercise,
        answer: &serde_json::Value,
    ) -> Result<Evaluation, AssessmentError> {
        match &exercise.exercise_type {
            models::exercise::ExerciseType::MultipleChoice {
                options: _,
                correct_index,
            } => evaluation::multiple_choice::evaluate(exercise, answer, *correct_index),
            models::exercise::ExerciseType::ShortAnswer {
                model_answer,
                key_points,
            } => evaluation::short_answer::evaluate(exercise, answer, model_answer, key_points),
            models::exercise::ExerciseType::Coding {
                language,
                test_cases,
                starter_code: _,
            } => evaluation::coding::evaluate(exercise, answer, language, test_cases),
            models::exercise::ExerciseType::Reflection {
                prompt: _,
                min_length,
                rubric_dimensions,
            } => evaluation::reflection::evaluate(exercise, answer, *min_length, rubric_dimensions),
        }
    }
}

impl Default for AssessmentEngine {
    fn default() -> Self {
        Self::new()
    }
}
