use crate::models::evaluation::Evaluation;

pub struct Scorer;

impl Scorer {
    pub fn new() -> Self {
        Self
    }

    pub fn calculate_score(evaluation: &Evaluation) -> f64 {
        evaluation.score
    }

    pub fn calculate_percentage(evaluation: &Evaluation) -> f64 {
        if evaluation.max_score == 0.0 {
            return 0.0;
        }
        (evaluation.score / evaluation.max_score) * 100.0
    }

    pub fn is_passing(evaluation: &Evaluation, threshold: f64) -> bool {
        let percentage = Self::calculate_percentage(evaluation);
        percentage >= threshold * 100.0
    }

    pub fn grade_letter(evaluation: &Evaluation) -> char {
        let percentage = Self::calculate_percentage(evaluation);
        match percentage as u32 {
            90..=100 => 'A',
            80..=89 => 'B',
            70..=79 => 'C',
            60..=69 => 'D',
            _ => 'F',
        }
    }
}

impl Default for Scorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn test_score_calculation() {
        let evaluation =
            Evaluation::new(Uuid::new_v4(), json!({}), 8.0, 10.0, "Good job".to_string());

        assert_eq!(Scorer::calculate_score(&evaluation), 8.0);
        assert_eq!(Scorer::calculate_percentage(&evaluation), 80.0);
        assert!(Scorer::is_passing(&evaluation, 0.7));
        assert_eq!(Scorer::grade_letter(&evaluation), 'B');
    }

    #[test]
    fn test_perfect_score() {
        let evaluation =
            Evaluation::new(Uuid::new_v4(), json!({}), 10.0, 10.0, "Perfect".to_string());

        assert_eq!(Scorer::calculate_percentage(&evaluation), 100.0);
        assert_eq!(Scorer::grade_letter(&evaluation), 'A');
    }

    #[test]
    fn test_failing_score() {
        let evaluation = Evaluation::new(
            Uuid::new_v4(),
            json!({}),
            3.0,
            10.0,
            "Needs improvement".to_string(),
        );

        assert_eq!(Scorer::calculate_percentage(&evaluation), 30.0);
        assert!(!Scorer::is_passing(&evaluation, 0.7));
        assert_eq!(Scorer::grade_letter(&evaluation), 'F');
    }
}
