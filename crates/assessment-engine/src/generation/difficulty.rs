use crate::models::exercise::Difficulty;

pub struct DifficultyCalibrator;

impl DifficultyCalibrator {
    pub fn calibrate(difficulty: &Difficulty, learner_level: &str) -> f64 {
        match (difficulty, learner_level) {
            (Difficulty::Easy, "beginner") => 0.8,
            (Difficulty::Easy, "intermediate") => 0.9,
            (Difficulty::Easy, "advanced") => 1.0,
            (Difficulty::Medium, "beginner") => 0.6,
            (Difficulty::Medium, "intermediate") => 0.8,
            (Difficulty::Medium, "advanced") => 0.9,
            (Difficulty::Hard, "beginner") => 0.4,
            (Difficulty::Hard, "intermediate") => 0.6,
            (Difficulty::Hard, "advanced") => 0.8,
            _ => 0.7, // Default
        }
    }

    pub fn adjust_max_score(base_score: f64, difficulty: &Difficulty, learner_level: &str) -> f64 {
        let multiplier = Self::calibrate(difficulty, learner_level);
        base_score * multiplier
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beginner_easy() {
        let score = DifficultyCalibrator::calibrate(&Difficulty::Easy, "beginner");
        assert_eq!(score, 0.8);
    }

    #[test]
    fn test_advanced_hard() {
        let score = DifficultyCalibrator::calibrate(&Difficulty::Hard, "advanced");
        assert_eq!(score, 0.8);
    }

    #[test]
    fn test_beginner_hard_is_hardest() {
        let score = DifficultyCalibrator::calibrate(&Difficulty::Hard, "beginner");
        assert_eq!(score, 0.4); // Lowest calibration
    }

    #[test]
    fn test_unknown_level_defaults() {
        let score = DifficultyCalibrator::calibrate(&Difficulty::Medium, "unknown");
        assert_eq!(score, 0.7); // Default fallback
    }

    #[test]
    fn test_adjust_max_score() {
        let adjusted =
            DifficultyCalibrator::adjust_max_score(10.0, &Difficulty::Medium, "beginner");
        assert!((adjusted - 6.0).abs() < 1e-10); // 10.0 * 0.6
    }

    #[test]
    fn test_all_difficulty_level_combinations() {
        let difficulties = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];
        let levels = ["beginner", "intermediate", "advanced"];

        for d in &difficulties {
            for l in &levels {
                let score = DifficultyCalibrator::calibrate(d, l);
                assert!(
                    (0.0..=1.0).contains(&score),
                    "Score {score} out of range for {d:?}/{l}"
                );
            }
        }
    }
}
