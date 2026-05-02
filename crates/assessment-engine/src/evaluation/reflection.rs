use crate::error::AssessmentError;
use crate::models::evaluation::{Evaluation, RubricResult};
use crate::models::exercise::{Exercise, RubricDimension};

pub fn evaluate(
    exercise: &Exercise,
    answer: &serde_json::Value,
    min_length: usize,
    rubric_dimensions: &[RubricDimension],
) -> Result<Evaluation, AssessmentError> {
    let reflection = answer
        .get("reflection")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AssessmentError::InvalidAnswer("Missing 'reflection' field".to_string()))?;

    if reflection.is_empty() {
        return Ok(Evaluation::new(
            exercise.id,
            answer.clone(),
            0.0,
            exercise.max_score,
            "No reflection provided.".to_string(),
        ));
    }

    // Check minimum length
    if reflection.len() < min_length {
        return Ok(Evaluation::new(
            exercise.id,
            answer.clone(),
            0.0,
            exercise.max_score,
            format!(
                "Reflection is too short. Minimum length is {} characters, but you provided {}.",
                min_length,
                reflection.len()
            ),
        ));
    }

    // Evaluate against rubric dimensions
    let mut rubric_results = Vec::new();
    let mut total_score = 0.0;

    for dimension in rubric_dimensions {
        let dimension_score = evaluate_dimension(reflection, dimension);
        let weighted_score = dimension_score * dimension.max_score;

        rubric_results.push(RubricResult {
            dimension: dimension.name.clone(),
            score: weighted_score,
            max_score: dimension.max_score,
            comment: format!(
                "Scored {:.1} out of {:.1}",
                weighted_score, dimension.max_score
            ),
        });

        total_score += weighted_score;
    }

    let score = if rubric_dimensions.is_empty() {
        // If no rubric dimensions, give full score for meeting length requirement
        exercise.max_score
    } else {
        total_score
    };

    let is_correct = score >= exercise.max_score * 0.7;

    let feedback = if is_correct {
        "Great reflection! You've demonstrated good understanding.".to_string()
    } else {
        "Your reflection could be more detailed. Consider expanding on the key points.".to_string()
    };

    Ok(Evaluation::new(
        exercise.id,
        answer.clone(),
        score,
        exercise.max_score,
        feedback,
    )
    .with_rubric_results(rubric_results))
}

fn evaluate_dimension(reflection: &str, dimension: &RubricDimension) -> f64 {
    // Simple heuristic: check if reflection mentions key terms from dimension
    let reflection_lower = reflection.to_lowercase();
    let dimension_lower = dimension.description.to_lowercase();

    // Count keyword matches
    let keywords: Vec<&str> = dimension_lower
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .collect();

    if keywords.is_empty() {
        return 0.8; // Default score if no keywords
    }

    let matches = keywords
        .iter()
        .filter(|kw| reflection_lower.contains(*kw))
        .count();

    let match_ratio = matches as f64 / keywords.len() as f64;

    // Scale to 0.0 - 1.0
    match_ratio.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::exercise::Exercise;

    #[test]
    fn test_good_reflection() {
        let dimensions = vec![RubricDimension {
            name: "understanding".to_string(),
            description: "understanding concept".to_string(),
            max_score: 2.0,
        }];

        let exercise = Exercise {
            id: uuid::Uuid::new_v4(),
            chapter_id: "ch1".to_string(),
            question: "Reflect on what you learned".to_string(),
            exercise_type: crate::models::exercise::ExerciseType::Reflection {
                prompt: "Write about what you learned".to_string(),
                min_length: 50,
                rubric_dimensions: dimensions.clone(),
            },
            difficulty: crate::models::exercise::Difficulty::Medium,
            rubric: None,
            max_score: 2.0,
            hints: Vec::new(),
            explanation: None,
        };

        let answer = serde_json::json!({
            "reflection": "I learned that Rust is a systems programming language that focuses on memory safety and performance. The concept of ownership is central to Rust's design. This understanding helps me write better code."
        });

        let result = evaluate(&exercise, &answer, 50, &dimensions).unwrap();

        assert!(result.score > 0.0);
        assert!(result.is_correct);
    }

    #[test]
    fn test_short_reflection() {
        let dimensions = vec![];
        let exercise = Exercise {
            id: uuid::Uuid::new_v4(),
            chapter_id: "ch1".to_string(),
            question: "Reflect on what you learned".to_string(),
            exercise_type: crate::models::exercise::ExerciseType::Reflection {
                prompt: "Write about what you learned".to_string(),
                min_length: 100,
                rubric_dimensions: dimensions.clone(),
            },
            difficulty: crate::models::exercise::Difficulty::Medium,
            rubric: None,
            max_score: 2.0,
            hints: Vec::new(),
            explanation: None,
        };

        let answer = serde_json::json!({
            "reflection": "Short reflection"
        });

        let result = evaluate(&exercise, &answer, 100, &dimensions).unwrap();

        assert_eq!(result.score, 0.0);
        assert!(!result.is_correct);
        assert!(result.feedback.contains("too short"));
    }

    #[test]
    fn test_empty_reflection() {
        let dimensions = vec![];
        let exercise = Exercise {
            id: uuid::Uuid::new_v4(),
            chapter_id: "ch1".to_string(),
            question: "Reflect on what you learned".to_string(),
            exercise_type: crate::models::exercise::ExerciseType::Reflection {
                prompt: "Write about what you learned".to_string(),
                min_length: 50,
                rubric_dimensions: dimensions.clone(),
            },
            difficulty: crate::models::exercise::Difficulty::Medium,
            rubric: None,
            max_score: 2.0,
            hints: Vec::new(),
            explanation: None,
        };

        let answer = serde_json::json!({"reflection": ""});
        let result = evaluate(&exercise, &answer, 50, &dimensions).unwrap();

        assert_eq!(result.score, 0.0);
        assert!(!result.is_correct);
    }
}
