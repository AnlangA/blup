use std::path::Path;
use uuid::Uuid;

use super::metadata::detect_language;
use crate::error::ImportError;
use crate::models::{ExtractionMethod, SourceChunk, SourceDocument, SourceMetadata, SourceType};

pub async fn import_markdown(path: &Path) -> Result<SourceDocument, ImportError> {
    // 1. Validate file exists
    if !path.exists() {
        return Err(ImportError::FileNotFound {
            path: path.to_string_lossy().to_string(),
        });
    }

    // 2. Read file content
    let content = tokio::fs::read_to_string(path).await?;

    // 3. Parse and chunk markdown
    let chunks = parse_markdown_with_headings(&content);

    // 4. Compute checksum
    let checksum = compute_checksum(content.as_bytes());

    // 5. Detect language
    let language = detect_language(&content);

    // 6. Build document
    let doc_id = Uuid::new_v4();
    let source_chunks: Vec<SourceChunk> = chunks
        .into_iter()
        .enumerate()
        .map(|(i, (heading_path, text))| SourceChunk {
            id: Uuid::new_v4(),
            document_id: doc_id,
            index: i as u32,
            content: text.clone(),
            heading_path,
            token_count: estimate_token_count(&text),
            overlap_with_previous: false,
        })
        .collect();

    let word_count = content.split_whitespace().count() as u32;

    Ok(SourceDocument {
        id: doc_id,
        source_type: SourceType::Markdown,
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
            character_count: content.len() as u32,
            extraction_method: ExtractionMethod::MarkdownParse,
            extraction_confidence: 1.0,
            ocr_applied: false,
            warnings: Vec::new(),
        },
        chunks: source_chunks,
    })
}

pub fn parse_markdown_with_headings(content: &str) -> Vec<(Vec<String>, String)> {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};

    let mut chunks = Vec::new();
    let mut current_heading_path: Vec<String> = Vec::new();
    let mut current_content = String::new();
    let mut heading_level: usize = 0;

    let parser = Parser::new(content);

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // Save previous chunk if exists
                if !current_content.trim().is_empty() {
                    chunks.push((
                        current_heading_path.clone(),
                        current_content.trim().to_string(),
                    ));
                    current_content.clear();
                }

                // Update heading level
                heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };

                // Trim heading path to current level
                current_heading_path.truncate(heading_level.saturating_sub(1));
            }
            Event::End(TagEnd::Heading(_)) => {
                // Heading text will be captured in the next Text event
            }
            Event::Text(text) => {
                if heading_level > 0 && current_heading_path.len() < heading_level {
                    current_heading_path.push(text.to_string());
                    heading_level = 0;
                } else {
                    current_content.push_str(&text);
                }
            }
            Event::Code(code) => {
                current_content.push_str(&format!("`{}`", code));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                current_content.push_str("```\n");
            }
            Event::End(TagEnd::CodeBlock) => {
                current_content.push_str("\n```\n");
            }
            Event::SoftBreak | Event::HardBreak => {
                current_content.push('\n');
            }
            _ => {}
        }
    }

    // Save last chunk
    if !current_content.trim().is_empty() {
        chunks.push((current_heading_path, current_content.trim().to_string()));
    }

    chunks
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
