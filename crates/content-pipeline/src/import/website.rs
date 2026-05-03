use uuid::Uuid;

use super::chunker::{chunk_text, ChunkConfig};
use super::metadata::detect_language;
use crate::error::ImportError;
use crate::models::{ExtractionMethod, SourceChunk, SourceDocument, SourceMetadata, SourceType};

pub async fn import_website(url: &str) -> Result<SourceDocument, ImportError> {
    // 1. Validate URL
    let parsed = url::Url::parse(url).map_err(|_| ImportError::InvalidUrl(url.to_string()))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ImportError::InvalidUrl(url.to_string()));
    }

    // 2. Security: reject internal/private URLs
    let host = parsed
        .host_str()
        .ok_or_else(|| ImportError::InvalidUrl(url.to_string()))?;
    if is_private_host(host) {
        return Err(ImportError::UrlBlocked {
            url: url.to_string(),
            reason: "Cannot import from internal/private URLs".to_string(),
        });
    }

    // 3. Fetch URL content
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::limited(5))
        .user_agent("Blup-ContentImporter/1.0")
        .build()
        .map_err(|e| ImportError::FetchFailed {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| ImportError::FetchFailed {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    if !response.status().is_success() {
        return Err(ImportError::FetchFailed {
            url: url.to_string(),
            reason: format!("HTTP {}", response.status()),
        });
    }

    if let Some(final_host) = response.url().host_str() {
        if is_private_host(final_host) {
            return Err(ImportError::UrlBlocked {
                url: response.url().to_string(),
                reason: "Cannot import from internal/private URLs after redirects".to_string(),
            });
        }
    }

    let html = response
        .text()
        .await
        .map_err(|e| ImportError::FetchFailed {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    // 4. Extract main content
    let document = scraper::Html::parse_document(&html);
    let mut content = String::new();

    // Try to find main content area
    let main_selectors = ["article", "main", "[role='main']", ".content", "#content"];
    let mut found_main = false;

    for selector_str in &main_selectors {
        if let Ok(selector) = scraper::Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                content = element.text().collect::<Vec<_>>().join(" ");
                found_main = true;
                break;
            }
        }
    }

    // Fallback to body text
    if !found_main {
        if let Ok(selector) = scraper::Selector::parse("body") {
            if let Some(body) = document.select(&selector).next() {
                content = body.text().collect::<Vec<_>>().join(" ");
            }
        }
    }

    // Clean up whitespace
    content = clean_whitespace(&content);

    if content.len() < 100 {
        return Err(ImportError::ContentTooShort {
            origin: url.to_string(),
            length: content.len(),
        });
    }

    // 5. Extract title
    let title = if let Ok(selector) = scraper::Selector::parse("title") {
        document
            .select(&selector)
            .next()
            .map(|t| t.inner_html())
            .unwrap_or_else(|| url.to_string())
    } else {
        url.to_string()
    };

    // 6. Compute checksum
    let checksum = compute_checksum(content.as_bytes());

    // 7. Detect language
    let language = detect_language(&content);

    // 8. Chunk text
    let chunk_config = ChunkConfig::default();
    let chunks = chunk_text(&content, &chunk_config);

    // 9. Build document
    let doc_id = Uuid::new_v4();
    let source_chunks: Vec<SourceChunk> = chunks
        .into_iter()
        .enumerate()
        .map(|(i, chunk_content)| SourceChunk {
            id: Uuid::new_v4(),
            document_id: doc_id,
            index: i as u32,
            content: chunk_content.clone(),
            heading_path: Vec::new(),
            token_count: estimate_token_count(&chunk_content),
            overlap_with_previous: i > 0,
        })
        .collect();

    let word_count = content.split_whitespace().count() as u32;

    Ok(SourceDocument {
        id: doc_id,
        source_type: SourceType::Website,
        title,
        origin: url.to_string(),
        checksum,
        language,
        license_or_usage_note: None,
        extracted_at: chrono::Utc::now(),
        metadata: SourceMetadata {
            page_count: None,
            word_count,
            character_count: content.len() as u32,
            extraction_method: ExtractionMethod::FetchReadability,
            extraction_confidence: 0.9,
            ocr_applied: false,
            warnings: Vec::new(),
        },
        chunks: source_chunks,
    })
}

fn is_private_host(host: &str) -> bool {
    host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("192.168.")
        || host.starts_with("10.")
        || host.starts_with("172.16.")
        || host.starts_with("172.17.")
        || host.starts_with("172.18.")
        || host.starts_with("172.19.")
        || host.starts_with("172.20.")
        || host.starts_with("172.21.")
        || host.starts_with("172.22.")
        || host.starts_with("172.23.")
        || host.starts_with("172.24.")
        || host.starts_with("172.25.")
        || host.starts_with("172.26.")
        || host.starts_with("172.27.")
        || host.starts_with("172.28.")
        || host.starts_with("172.29.")
        || host.starts_with("172.30.")
        || host.starts_with("172.31.")
        || host.starts_with("169.254.")
        || host.starts_with("0.")
}

fn clean_whitespace(text: &str) -> String {
    let mut result = String::new();
    let mut prev_was_space = false;

    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
    }

    result.trim().to_string()
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
