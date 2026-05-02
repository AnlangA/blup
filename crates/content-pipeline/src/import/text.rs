use std::path::Path;
use uuid::Uuid;

use super::chunker::{chunk_text, ChunkConfig};
use super::metadata::detect_language;
use crate::error::ImportError;
use crate::models::{ExtractionMethod, SourceChunk, SourceDocument, SourceMetadata, SourceType};

pub async fn import_text(path: &Path) -> Result<SourceDocument, ImportError> {
    // 1. Validate file exists
    if !path.exists() {
        return Err(ImportError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    // 2. Read file bytes
    let bytes = tokio::fs::read(path).await?;

    // 3. Detect encoding and decode
    let (text, encoding) = detect_encoding_and_decode(&bytes)?;

    // 4. Compute checksum
    let checksum = compute_checksum(&bytes);

    // 5. Detect language
    let language = detect_language(&text);

    // 6. Chunk text
    let chunk_config = ChunkConfig::default();
    let chunks = chunk_text(&text, &chunk_config);

    // 7. Build document
    let doc_id = Uuid::new_v4();
    let source_chunks: Vec<SourceChunk> = chunks
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

    let mut warnings = Vec::new();
    if encoding != "UTF-8" {
        warnings.push(format!("Converted from {} encoding", encoding));
    }

    Ok(SourceDocument {
        id: doc_id,
        source_type: SourceType::PlainText,
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
            extraction_method: ExtractionMethod::TextRead,
            extraction_confidence: 1.0,
            ocr_applied: false,
            warnings,
        },
        chunks: source_chunks,
    })
}

fn detect_encoding_and_decode(bytes: &[u8]) -> Result<(String, String), ImportError> {
    // Check BOM
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        let text = String::from_utf8_lossy(&bytes[3..]).to_string();
        return Ok((text, "UTF-8".to_string()));
    }
    if bytes.starts_with(&[0xFE, 0xFF]) {
        let (text, _, _) = encoding_rs::UTF_16BE.decode(bytes);
        return Ok((text.to_string(), "UTF-16BE".to_string()));
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        let (text, _, _) = encoding_rs::UTF_16LE.decode(bytes);
        return Ok((text.to_string(), "UTF-16LE".to_string()));
    }

    // Try UTF-8 first
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok((text.to_string(), "UTF-8".to_string()));
    }

    // Try Latin-1 as fallback
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
    Ok((text.to_string(), "WINDOWS-1252".to_string()))
}

fn compute_checksum(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

fn estimate_token_count(text: &str) -> u32 {
    (text.len() as u32) / 4
}
