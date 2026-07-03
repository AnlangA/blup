use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
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
    ensure_public_host(host, parsed.port_or_known_default(), url).await?;

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
        ensure_public_host(
            final_host,
            response.url().port_or_known_default(),
            response.url().as_str(),
        )
        .await?;
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

async fn ensure_public_host(host: &str, port: Option<u16>, url: &str) -> Result<(), ImportError> {
    if is_private_host(host) {
        return Err(ImportError::UrlBlocked {
            url: url.to_string(),
            reason: "Cannot import from internal/private URLs".to_string(),
        });
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(ip) {
            return Err(ImportError::UrlBlocked {
                url: url.to_string(),
                reason: "Cannot import from private, loopback, link-local, multicast, or documentation IP ranges".to_string(),
            });
        }
        return Ok(());
    }

    let port = port.unwrap_or(80);
    let resolved =
        tokio::net::lookup_host((host, port))
            .await
            .map_err(|e| ImportError::FetchFailed {
                url: url.to_string(),
                reason: format!("DNS lookup failed: {e}"),
            })?;

    for addr in resolved {
        if is_blocked_ip(addr.ip()) {
            return Err(ImportError::UrlBlocked {
                url: url.to_string(),
                reason: "Hostname resolves to a private, loopback, link-local, multicast, or documentation IP range".to_string(),
            });
        }
    }

    Ok(())
}

fn is_blocked_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_blocked_ipv4(ip),
        IpAddr::V6(ip) => is_blocked_ipv6(ip),
    }
}

fn is_blocked_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private()
        || ip.is_loopback()
        || ip.is_link_local()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_multicast()
        || ip.is_unspecified()
}

fn is_blocked_ipv6(ip: Ipv6Addr) -> bool {
    ip.is_loopback()
        || ip.is_multicast()
        || ip.is_unspecified()
        || is_ipv6_unique_local(ip)
        || is_ipv6_unicast_link_local(ip)
        || is_ipv6_documentation(ip)
}

fn is_ipv6_unique_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

fn is_ipv6_unicast_link_local(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xffc0) == 0xfe80
}

fn is_ipv6_documentation(ip: Ipv6Addr) -> bool {
    ip.segments()[0] == 0x2001 && ip.segments()[1] == 0x0db8
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_private_and_loopback_ipv4_ranges() {
        for ip in [
            "127.0.0.1",
            "10.0.0.5",
            "172.16.1.1",
            "192.168.1.1",
            "169.254.1.1",
        ] {
            assert!(is_blocked_ip(ip.parse().unwrap()), "{ip} should be blocked");
        }
    }

    #[test]
    fn blocks_private_and_link_local_ipv6_ranges() {
        for ip in ["::1", "fc00::1", "fd12::1", "fe80::1", "2001:db8::1"] {
            assert!(is_blocked_ip(ip.parse().unwrap()), "{ip} should be blocked");
        }
    }

    #[test]
    fn allows_public_ips() {
        for ip in ["8.8.8.8", "2606:4700:4700::1111"] {
            assert!(
                !is_blocked_ip(ip.parse().unwrap()),
                "{ip} should be allowed"
            );
        }
    }
}
