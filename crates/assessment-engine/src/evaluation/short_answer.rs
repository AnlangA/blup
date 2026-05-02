use crate::error::AssessmentError;
use crate::models::evaluation::{Evaluation, RubricResult};
use crate::models::exercise::Exercise;

pub fn evaluate(
    exercise: &Exercise,
    answer: &serde_json::Value,
    _model_answer: &str,
    key_points: &[String],
) -> Result<Evaluation, AssessmentError> {
    let learner_answer = answer
        .get("answer")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AssessmentError::InvalidAnswer("Missing 'answer' field".to_string()))?;

    if learner_answer.is_empty() {
        return Ok(Evaluation::new(
            exercise.id,
            answer.clone(),
            0.0,
            exercise.max_score,
            "No answer provided.".to_string(),
        ));
    }

    // Simple keyword matching for key points
    let (matched_points, missed_points) = match_key_points(learner_answer, key_points);

    let score = if key_points.is_empty() {
        exercise.max_score
    } else {
        (matched_points.len() as f64 / key_points.len() as f64) * exercise.max_score
    };

    let is_correct = score >= exercise.max_score * 0.7;

    let mut feedback = String::new();
    if is_correct {
        feedback.push_str("Good answer! ");
    } else {
        feedback.push_str("Your answer covers some key points but misses others. ");
    }

    if !matched_points.is_empty() {
        feedback.push_str(&format!("You covered: {}. ", matched_points.join(", ")));
    }
    if !missed_points.is_empty() {
        feedback.push_str(&format!(
            "Consider also mentioning: {}. ",
            missed_points.join(", ")
        ));
    }

    let rubric_results = vec![RubricResult {
        dimension: "key_points_coverage".to_string(),
        score: matched_points.len() as f64,
        max_score: key_points.len() as f64,
        comment: format!(
            "{} of {} key points covered",
            matched_points.len(),
            key_points.len()
        ),
    }];

    Ok(Evaluation::new(
        exercise.id,
        answer.clone(),
        score,
        exercise.max_score,
        feedback,
    )
    .with_rubric_results(rubric_results))
}

fn match_key_points(learner_answer: &str, key_points: &[String]) -> (Vec<String>, Vec<String>) {
    let learner_lower = learner_answer.to_lowercase();
    let mut matched = Vec::new();
    let mut missed = Vec::new();

    for point in key_points {
        let keywords: Vec<&str> = point.split_whitespace().filter(|w| w.len() > 3).collect();

        let match_count = keywords
            .iter()
            .filter(|kw| learner_lower.contains(&kw.to_lowercase()))
            .count();

        if !keywords.is_empty() && (match_count as f64 / keywords.len() as f64) > 0.5 {
            matched.push(point.clone());
        } else {
            missed.push(point.clone());
        }
    }

    (matched, missed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::exercise::Exercise;

    #[test]
    fn test_good_answer() {
        let exercise = Exercise::new_short_answer(
            "ch1",
            "Explain what Rust is",
            "Rust is a systems programming language",
            vec![
                "systems programming".to_string(),
                "memory safety".to_string(),
                "performance".to_string(),
            ],
            3.0,
        );

        let answer = serde_json::json!({
            "answer": "Rust is a systems programming language focused on memory safety and performance"
        });

        let result = evaluate(
            &exercise,
            &answer,
            "Rust is a systems programming language",
            &[
                "systems programming".to_string(),
                "memory safety".to_string(),
                "performance".to_string(),
            ],
        )
        .unwrap();

        assert!(result.score > 0.0);
        assert!(result.is_correct);
    }

    #[test]
    fn test_empty_answer() {
        let exercise = Exercise::new_short_answer(
            "ch1",
            "Explain what Rust is",
            "Rust is a systems programming language",
            vec!["systems programming".to_string()],
            1.0,
        );

        let answer = serde_json::json!({"answer": ""});
        let result = evaluate(
            &exercise,
            &answer,
            "Rust is a systems programming language",
            &["systems programming".to_string()],
        )
        .unwrap();

        assert_eq!(result.score, 0.0);
        assert!(!result.is_correct);
    }

    #[test]
    fn test_invalid_answer_format() {
        let exercise = Exercise::new_short_answer(
            "ch1",
            "Explain what Rust is",
            "Rust is a systems programming language",
            vec!["systems programming".to_string()],
            1.0,
        );

        let answer = serde_json::json!({"wrong_field": "test"});
        let result = evaluate(
            &exercise,
            &answer,
            "Rust is a systems programming language",
            &["systems programming".to_string()],
        );

        assert!(result.is_err());
    }
}
