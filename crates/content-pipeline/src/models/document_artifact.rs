use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentArtifact {
    pub id: Uuid,
    pub session_id: Option<Uuid>,
    pub format: ArtifactFormat,
    pub checksum: String,
    pub size_bytes: u64,
    pub page_count: Option<u32>,
    pub generated_at: DateTime<Utc>,
    pub source_content_ids: Vec<Uuid>,
    pub source_typst: Option<String>,
    #[serde(skip)]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactFormat {
    Pdf,
    Typst,
}

impl std::fmt::Display for ArtifactFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactFormat::Pdf => write!(f, "pdf"),
            ArtifactFormat::Typst => write!(f, "typst"),
        }
    }
}

impl DocumentArtifact {
    pub fn new_pdf(data: &[u8], typst_source: &str) -> Self {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(data);
        let checksum = format!("sha256:{}", hex::encode(hasher.finalize()));

        Self {
            id: Uuid::new_v4(),
            session_id: None,
            format: ArtifactFormat::Pdf,
            checksum,
            size_bytes: data.len() as u64,
            page_count: None,
            generated_at: Utc::now(),
            source_content_ids: Vec::new(),
            source_typst: Some(typst_source.to_string()),
            data: data.to_vec(),
        }
    }

    pub fn new_typst(source: &str) -> Self {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
        let checksum = format!("sha256:{}", hex::encode(hasher.finalize()));

        Self {
            id: Uuid::new_v4(),
            session_id: None,
            format: ArtifactFormat::Typst,
            checksum,
            size_bytes: source.len() as u64,
            page_count: None,
            generated_at: Utc::now(),
            source_content_ids: Vec::new(),
            source_typst: Some(source.to_string()),
            data: Vec::new(),
        }
    }
}
