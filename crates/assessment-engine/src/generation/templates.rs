use crate::models::exercise::Difficulty;

pub struct ExerciseTemplates;

impl ExerciseTemplates {
    pub fn multiple_choice_template() -> ExerciseTemplate {
        ExerciseTemplate {
            template_type: "multiple_choice".to_string(),
            description: "Multiple choice question with 4 options".to_string(),
            default_max_score: 1.0,
            default_difficulty: Difficulty::Medium,
        }
    }

    pub fn short_answer_template() -> ExerciseTemplate {
        ExerciseTemplate {
            template_type: "short_answer".to_string(),
            description: "Short answer question with key points".to_string(),
            default_max_score: 2.0,
            default_difficulty: Difficulty::Medium,
        }
    }

    pub fn coding_template() -> ExerciseTemplate {
        ExerciseTemplate {
            template_type: "coding".to_string(),
            description: "Coding exercise with test cases".to_string(),
            default_max_score: 3.0,
            default_difficulty: Difficulty::Hard,
        }
    }

    pub fn reflection_template() -> ExerciseTemplate {
        ExerciseTemplate {
            template_type: "reflection".to_string(),
            description: "Reflection prompt with rubric dimensions".to_string(),
            default_max_score: 2.0,
            default_difficulty: Difficulty::Medium,
        }
    }
}

pub struct ExerciseTemplate {
    pub template_type: String,
    pub description: String,
    pub default_max_score: f64,
    pub default_difficulty: Difficulty,
}
