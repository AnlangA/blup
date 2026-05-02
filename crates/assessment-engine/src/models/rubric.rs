use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rubric {
    pub dimensions: Vec<RubricDimension>,
    pub passing_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubricDimension {
    pub name: String,
    pub description: String,
    pub max_score: f64,
    pub weight: f64,
}

impl Rubric {
    pub fn new(dimensions: Vec<RubricDimension>, passing_threshold: f64) -> Self {
        Self {
            dimensions,
            passing_threshold,
        }
    }

    pub fn total_max_score(&self) -> f64 {
        self.dimensions.iter().map(|d| d.max_score * d.weight).sum()
    }
}
