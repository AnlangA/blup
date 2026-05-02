pub mod error;
pub mod evaluation;
pub mod executor;
pub mod generation;
pub mod models;

use std::sync::Arc;

use error::AssessmentError;
use executor::CodeExecutor;
use models::evaluation::Evaluation;
use models::exercise::Exercise;

pub struct AssessmentEngine {
    code_executor: Option<Arc<dyn CodeExecutor>>,
}

impl Clone for AssessmentEngine {
    fn clone(&self) -> Self {
        Self {
            code_executor: self.code_executor.clone(),
        }
    }
}

impl AssessmentEngine {
    pub fn new() -> Self {
        Self {
            code_executor: None,
        }
    }

    /// Attach a code executor for running coding exercises in a sandbox.
    pub fn with_executor(mut self, executor: Arc<dyn CodeExecutor>) -> Self {
        self.code_executor = Some(executor);
        self
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
            } => evaluation::coding::evaluate(
                exercise,
                answer,
                language,
                test_cases,
                self.code_executor.as_deref(),
            ),
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
