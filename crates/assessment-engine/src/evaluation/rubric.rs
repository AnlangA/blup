use crate::models::evaluation::RubricResult;
use crate::models::rubric::Rubric;

pub fn evaluate_rubric(rubric: &Rubric, dimension_scores: &[(String, f64)]) -> Vec<RubricResult> {
    let mut results = Vec::new();

    for dimension in &rubric.dimensions {
        let score = dimension_scores
            .iter()
            .find(|(name, _)| name == &dimension.name)
            .map(|(_, score)| *score)
            .unwrap_or(0.0);

        let weighted_score = score * dimension.weight;

        results.push(RubricResult {
            dimension: dimension.name.clone(),
            score: weighted_score,
            max_score: dimension.max_score,
            comment: format!(
                "Scored {:.1} out of {:.1}",
                weighted_score, dimension.max_score
            ),
        });
    }

    results
}

pub fn calculate_total_score(rubric: &Rubric, dimension_scores: &[(String, f64)]) -> f64 {
    let mut total = 0.0;

    for dimension in &rubric.dimensions {
        let score = dimension_scores
            .iter()
            .find(|(name, _)| name == &dimension.name)
            .map(|(_, score)| *score)
            .unwrap_or(0.0);

        total += score * dimension.weight * dimension.max_score;
    }

    total
}

pub fn is_passing(rubric: &Rubric, total_score: f64) -> bool {
    let max_possible = rubric.total_max_score();
    total_score >= max_possible * rubric.passing_threshold
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::rubric::RubricDimension;

    #[test]
    fn test_rubric_evaluation() {
        let rubric = Rubric::new(
            vec![
                RubricDimension {
                    name: "content".to_string(),
                    description: "Quality of content".to_string(),
                    max_score: 3.0,
                    weight: 1.0,
                },
                RubricDimension {
                    name: "clarity".to_string(),
                    description: "Clarity of expression".to_string(),
                    max_score: 2.0,
                    weight: 0.5,
                },
            ],
            0.7,
        );

        let scores = vec![("content".to_string(), 0.8), ("clarity".to_string(), 0.9)];

        let results = evaluate_rubric(&rubric, &scores);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].dimension, "content");
        // 0.8 * 1.0 = 0.8
        assert!((results[0].score - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_total_score_calculation() {
        let rubric = Rubric::new(
            vec![RubricDimension {
                name: "content".to_string(),
                description: "Quality of content".to_string(),
                max_score: 4.0,
                weight: 1.0,
            }],
            0.7,
        );

        let scores = vec![("content".to_string(), 0.75)];
        let total = calculate_total_score(&rubric, &scores);
        assert!((total - 3.0).abs() < 0.01); // 0.75 * 4.0
    }

    #[test]
    fn test_passing_threshold() {
        let rubric = Rubric::new(
            vec![RubricDimension {
                name: "test".to_string(),
                description: "Test dimension".to_string(),
                max_score: 10.0,
                weight: 1.0,
            }],
            0.7,
        );

        assert!(is_passing(&rubric, 7.0));
        assert!(is_passing(&rubric, 8.0));
        assert!(!is_passing(&rubric, 6.0));
    }
}
