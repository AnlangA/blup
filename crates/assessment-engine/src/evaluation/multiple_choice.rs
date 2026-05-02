use crate::error::AssessmentError;
use crate::models::evaluation::Evaluation;
use crate::models::exercise::Exercise;

pub fn evaluate(
    exercise: &Exercise,
    answer: &serde_json::Value,
    correct_index: usize,
) -> Result<Evaluation, AssessmentError> {
    let selected = answer
        .get("selected_index")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| {
            AssessmentError::InvalidAnswer("Missing 'selected_index' field".to_string())
        })? as usize;

    let score = if selected == correct_index {
        exercise.max_score
    } else {
        0.0
    };

    let is_correct = selected == correct_index;

    let feedback = if is_correct {
        "Correct!".to_string()
    } else {
        format!(
            "The correct answer was option {}. {}",
            correct_index + 1,
            exercise.explanation.as_deref().unwrap_or("")
        )
    };

    Ok(Evaluation::new(
        exercise.id,
        answer.clone(),
        score,
        exercise.max_score,
        feedback,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correct_answer() {
        let exercise = Exercise::new_multiple_choice(
            "ch1",
            "What is 2+2?",
            vec!["3".to_string(), "4".to_string(), "5".to_string()],
            1,
            1.0,
        );

        let answer = serde_json::json!({"selected_index": 1});
        let result = evaluate(&exercise, &answer, 1).unwrap();

        assert_eq!(result.score, 1.0);
        assert!(result.is_correct);
        assert_eq!(result.feedback, "Correct!");
    }

    #[test]
    fn test_wrong_answer() {
        let exercise = Exercise::new_multiple_choice(
            "ch1",
            "What is 2+2?",
            vec!["3".to_string(), "4".to_string(), "5".to_string()],
            1,
            1.0,
        );

        let answer = serde_json::json!({"selected_index": 0});
        let result = evaluate(&exercise, &answer, 1).unwrap();

        assert_eq!(result.score, 0.0);
        assert!(!result.is_correct);
        assert!(result.feedback.contains("option 2"));
    }

    #[test]
    fn test_invalid_answer_format() {
        let exercise = Exercise::new_multiple_choice(
            "ch1",
            "What is 2+2?",
            vec!["3".to_string(), "4".to_string(), "5".to_string()],
            1,
            1.0,
        );

        let answer = serde_json::json!({"wrong_field": 1});
        let result = evaluate(&exercise, &answer, 1);

        assert!(result.is_err());
    }

    #[test]
    fn test_deterministic_evaluation() {
        let exercise = Exercise::new_multiple_choice(
            "ch1",
            "What is 2+2?",
            vec!["3".to_string(), "4".to_string(), "5".to_string()],
            1,
            1.0,
        );

        let answer = serde_json::json!({"selected_index": 1});

        // Run 100 times, should get same result
        for _ in 0..100 {
            let result = evaluate(&exercise, &answer, 1).unwrap();
            assert_eq!(result.score, 1.0);
            assert!(result.is_correct);
        }
    }
}
