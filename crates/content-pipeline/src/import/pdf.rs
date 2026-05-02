use std::path::Path;
use uuid::Uuid;

use super::chunker::{chunk_text, ChunkConfig};
use super::metadata::detect_language;
use crate::error::ImportError;
use crate::models::{ExtractionMethod, SourceChunk, SourceDocument, SourceMetadata, SourceType};

pub async fn import_pdf(path: &Path) -> Result<SourceDocument, ImportError> {
    // 1. Validate file exists
    if !path.exists() {
        return Err(ImportError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    // 2. Extract text using pdf-extract
    let text = pdf_extract::extract_text(path).map_err(|e| ImportError::ExtractionFailed {
        path: path.to_string_lossy().to_string(),
        reason: e.to_string(),
    })?;

    // 3. Check extraction quality
    if text.trim().is_empty() {
        return Err(ImportError::ExtractionFailed {
            path: path.to_string_lossy().to_string(),
            reason: "No text content extracted".to_string(),
        });
    }

    let confidence = text_quality_score(&text);
    if confidence < 0.3 {
        return Err(ImportError::ExtractionFailed {
            path: path.to_string_lossy().to_string(),
            reason: format!("Low extraction confidence: {:.2}", confidence),
        });
    }

    // 4. Compute checksum
    let checksum = compute_checksum(text.as_bytes());

    // 5. Detect language
    let language = detect_language(&text);

    // 6. Chunk text
    let chunk_config = ChunkConfig::default();
    let chunks = chunk_text(&text, &chunk_config);

    // 7. Build document
    let doc_id = Uuid::new_v4();
    let chunks: Vec<SourceChunk> = chunks
        .into_iter()
        .enumerate()
        .map(|(i, content)| SourceChunk {
            id: Uuid::new_v4(),
            document_id: doc_id,
            index: i as u32,
            content: content.clone(),
            heading_path: Vec::new(),
            token_count: estimate_token_count(&content),
            overlap_with_previous: i > 0,
        })
        .collect();

    let word_count = text.split_whitespace().count() as u32;

    Ok(SourceDocument {
        id: doc_id,
        source_type: SourceType::Pdf,
        title: path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string(),
        origin: path.to_string_lossy().to_string(),
        checksum,
        language,
        license_or_usage_note: None,
        extracted_at: chrono::Utc::now(),
        metadata: SourceMetadata {
            page_count: None,
            word_count,
            character_count: text.len() as u32,
            extraction_method: ExtractionMethod::DirectText,
            extraction_confidence: confidence,
            ocr_applied: false,
            warnings: Vec::new(),
        },
        chunks,
    })
}

fn text_quality_score(text: &str) -> f32 {
    let total = text.chars().count() as f32;
    if total == 0.0 {
        return 0.0;
    }
    let valid = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation())
        .count() as f32;
    valid / total
}

fn compute_checksum(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

fn estimate_token_count(text: &str) -> u32 {
    // Rough estimate: ~4 characters per token
    (text.len() as u32) / 4
}
