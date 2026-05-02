use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    pub name: String,
    pub tag: String,
    pub digest: Option<String>,
    pub size_mb: f64,
    pub created_at: Option<String>,
}

impl ImageInfo {
    pub fn new(name: &str, tag: &str) -> Self {
        Self {
            name: name.to_string(),
            tag: tag.to_string(),
            digest: None,
            size_mb: 0.0,
            created_at: None,
        }
    }

    pub fn with_digest(mut self, digest: &str) -> Self {
        self.digest = Some(digest.to_string());
        self
    }

    pub fn with_size(mut self, size_mb: f64) -> Self {
        self.size_mb = size_mb;
        self
    }

    pub fn full_name(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }
}
