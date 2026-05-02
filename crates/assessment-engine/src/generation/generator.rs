use crate::error::AssessmentError;
use crate::models::exercise::{Difficulty, Exercise, ExerciseType, RubricDimension, TestCase};
use uuid::Uuid;

pub struct ExerciseGenerator;

impl ExerciseGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate_multiple_choice(
        &self,
        chapter_id: &str,
        question: &str,
        options: Vec<String>,
        correct_index: usize,
        max_score: f64,
    ) -> Result<Exercise, AssessmentError> {
        if options.len() < 2 {
            return Err(AssessmentError::ValidationError(
                "Multiple choice must have at least 2 options".to_string(),
            ));
        }

        if correct_index >= options.len() {
            return Err(AssessmentError::ValidationError(
                "correct_index must be less than options length".to_string(),
            ));
        }

        Ok(Exercise::new_multiple_choice(
            chapter_id,
            question,
            options,
            correct_index,
            max_score,
        ))
    }

    pub fn generate_short_answer(
        &self,
        chapter_id: &str,
        question: &str,
        model_answer: &str,
        key_points: Vec<String>,
        max_score: f64,
    ) -> Result<Exercise, AssessmentError> {
        if model_answer.is_empty() {
            return Err(AssessmentError::ValidationError(
                "Model answer cannot be empty".to_string(),
            ));
        }

        Ok(Exercise::new_short_answer(
            chapter_id,
            question,
            model_answer,
            key_points,
            max_score,
        ))
    }

    pub fn generate_coding(
        &self,
        chapter_id: &str,
        question: &str,
        language: &str,
        test_cases: Vec<TestCase>,
        max_score: f64,
    ) -> Result<Exercise, AssessmentError> {
        if test_cases.is_empty() {
            return Err(AssessmentError::ValidationError(
                "Coding exercise must have at least one test case".to_string(),
            ));
        }

        Ok(Exercise::new_coding(
            chapter_id, question, language, test_cases, max_score,
        ))
    }

    pub fn generate_reflection(
        &self,
        chapter_id: &str,
        question: &str,
        prompt: &str,
        min_length: usize,
        rubric_dimensions: Vec<RubricDimension>,
        max_score: f64,
    ) -> Result<Exercise, AssessmentError> {
        if min_length == 0 {
            return Err(AssessmentError::ValidationError(
                "Minimum length must be greater than 0".to_string(),
            ));
        }

        Ok(Exercise {
            id: Uuid::new_v4(),
            chapter_id: chapter_id.to_string(),
            question: question.to_string(),
            exercise_type: ExerciseType::Reflection {
                prompt: prompt.to_string(),
                min_length,
                rubric_dimensions,
            },
            difficulty: Difficulty::Medium,
            rubric: None,
            max_score,
            hints: Vec::new(),
            explanation: None,
        })
    }
}

impl Default for ExerciseGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mc_valid() {
        let gen = ExerciseGenerator::new();
        let ex = gen
            .generate_multiple_choice(
                "ch1",
                "What is 2+2?",
                vec!["3".to_string(), "4".to_string()],
                1,
                1.0,
            )
            .unwrap();
        assert_eq!(ex.chapter_id, "ch1");
        match ex.exercise_type {
            ExerciseType::MultipleChoice {
                options,
                correct_index,
            } => {
                assert_eq!(options.len(), 2);
                assert_eq!(correct_index, 1);
            }
            _ => panic!("Expected MultipleChoice"),
        }
    }

    #[test]
    fn test_generate_mc_too_few_options() {
        let gen = ExerciseGenerator::new();
        let result = gen.generate_multiple_choice("ch1", "Q?", vec!["Only".to_string()], 0, 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_mc_bad_index() {
        let gen = ExerciseGenerator::new();
        let result = gen.generate_multiple_choice(
            "ch1",
            "Q?",
            vec!["A".to_string(), "B".to_string()],
            5,
            1.0,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_short_answer_empty_model() {
        let gen = ExerciseGenerator::new();
        let result = gen.generate_short_answer("ch1", "Q?", "", vec!["kp".to_string()], 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_coding_no_test_cases() {
        let gen = ExerciseGenerator::new();
        let result = gen.generate_coding("ch1", "Q?", "python", vec![], 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_reflection_zero_min_length() {
        let gen = ExerciseGenerator::new();
        let result = gen.generate_reflection("ch1", "Q?", "Think", 0, vec![], 1.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_reflection_valid() {
        let gen = ExerciseGenerator::new();
        let dims = vec![RubricDimension {
            name: "clarity".to_string(),
            description: "Clear writing".to_string(),
            max_score: 2.0,
        }];
        let ex = gen
            .generate_reflection("ch1", "Q?", "Think deeply", 50, dims, 3.0)
            .unwrap();
        match ex.exercise_type {
            ExerciseType::Reflection {
                min_length,
                rubric_dimensions,
                ..
            } => {
                assert_eq!(min_length, 50);
                assert_eq!(rubric_dimensions.len(), 1);
            }
            _ => panic!("Expected Reflection"),
        }
    }
}
