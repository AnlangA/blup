use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDocument {
    pub id: Uuid,
    pub source_type: SourceType,
    pub title: String,
    pub origin: String,
    pub checksum: String,
    pub language: Option<String>,
    pub license_or_usage_note: Option<String>,
    pub extracted_at: DateTime<Utc>,
    pub metadata: SourceMetadata,
    pub chunks: Vec<SourceChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    Pdf,
    Markdown,
    PlainText,
    Website,
}

impl std::fmt::Display for SourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SourceType::Pdf => write!(f, "pdf"),
            SourceType::Markdown => write!(f, "markdown"),
            SourceType::PlainText => write!(f, "plain_text"),
            SourceType::Website => write!(f, "website"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetadata {
    pub page_count: Option<u32>,
    pub word_count: u32,
    pub character_count: u32,
    pub extraction_method: ExtractionMethod,
    pub extraction_confidence: f32,
    pub ocr_applied: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMethod {
    DirectText,
    Pdftotext,
    Ocr,
    FetchReadability,
    MarkdownParse,
    TextRead,
}

impl std::fmt::Display for ExtractionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractionMethod::DirectText => write!(f, "direct_text"),
            ExtractionMethod::Pdftotext => write!(f, "pdftotext"),
            ExtractionMethod::Ocr => write!(f, "ocr"),
            ExtractionMethod::FetchReadability => write!(f, "fetch_readability"),
            ExtractionMethod::MarkdownParse => write!(f, "markdown_parse"),
            ExtractionMethod::TextRead => write!(f, "text_read"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceChunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub index: u32,
    pub content: String,
    pub heading_path: Vec<String>,
    pub token_count: u32,
    pub overlap_with_previous: bool,
}

impl SourceDocument {
    pub fn total_words(&self) -> u32 {
        self.metadata.word_count
    }

    pub fn total_chars(&self) -> u32 {
        self.metadata.character_count
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }
}
