use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Exercise {
    pub id: Uuid,
    pub chapter_id: String,
    pub question: String,
    pub exercise_type: ExerciseType,
    pub difficulty: Difficulty,
    pub rubric: Option<serde_json::Value>,
    pub max_score: f64,
    pub hints: Vec<String>,
    pub explanation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ExerciseType {
    #[serde(rename = "multiple_choice")]
    MultipleChoice {
        options: Vec<String>,
        correct_index: usize,
    },
    #[serde(rename = "short_answer")]
    ShortAnswer {
        model_answer: String,
        key_points: Vec<String>,
    },
    #[serde(rename = "coding")]
    Coding {
        language: String,
        starter_code: Option<String>,
        test_cases: Vec<TestCase>,
    },
    #[serde(rename = "reflection")]
    Reflection {
        prompt: String,
        min_length: usize,
        rubric_dimensions: Vec<RubricDimension>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub input: String,
    pub expected_output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricDimension {
    pub name: String,
    pub description: String,
    pub max_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl ExerciseType {
    pub fn test_cases(&self) -> Vec<TestCase> {
        match self {
            ExerciseType::Coding { test_cases, .. } => test_cases.clone(),
            _ => Vec::new(),
        }
    }
}

impl Exercise {
    pub fn new_multiple_choice(
        chapter_id: &str,
        question: &str,
        options: Vec<String>,
        correct_index: usize,
        max_score: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            chapter_id: chapter_id.to_string(),
            question: question.to_string(),
            exercise_type: ExerciseType::MultipleChoice {
                options,
                correct_index,
            },
            difficulty: Difficulty::Medium,
            rubric: None,
            max_score,
            hints: Vec::new(),
            explanation: None,
        }
    }

    pub fn new_short_answer(
        chapter_id: &str,
        question: &str,
        model_answer: &str,
        key_points: Vec<String>,
        max_score: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            chapter_id: chapter_id.to_string(),
            question: question.to_string(),
            exercise_type: ExerciseType::ShortAnswer {
                model_answer: model_answer.to_string(),
                key_points,
            },
            difficulty: Difficulty::Medium,
            rubric: None,
            max_score,
            hints: Vec::new(),
            explanation: None,
        }
    }

    pub fn new_coding(
        chapter_id: &str,
        question: &str,
        language: &str,
        test_cases: Vec<TestCase>,
        max_score: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            chapter_id: chapter_id.to_string(),
            question: question.to_string(),
            exercise_type: ExerciseType::Coding {
                language: language.to_string(),
                starter_code: None,
                test_cases,
            },
            difficulty: Difficulty::Medium,
            rubric: None,
            max_score,
            hints: Vec::new(),
            explanation: None,
        }
    }
}
