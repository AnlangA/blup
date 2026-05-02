use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evaluation {
    pub exercise_id: Uuid,
    pub learner_answer: serde_json::Value,
    pub score: f64,
    pub max_score: f64,
    pub feedback: String,
    pub rubric_results: Vec<RubricResult>,
    pub is_correct: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricResult {
    pub dimension: String,
    pub score: f64,
    pub max_score: f64,
    pub comment: String,
}

impl Evaluation {
    pub fn new(
        exercise_id: Uuid,
        learner_answer: serde_json::Value,
        score: f64,
        max_score: f64,
        feedback: String,
    ) -> Self {
        let is_correct = score >= max_score * 0.7; // 70% threshold
        Self {
            exercise_id,
            learner_answer,
            score,
            max_score,
            feedback,
            rubric_results: Vec::new(),
            is_correct,
        }
    }

    pub fn with_rubric_results(mut self, rubric_results: Vec<RubricResult>) -> Self {
        self.rubric_results = rubric_results;
        self
    }
}
