/// Configuration for text chunking
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum chunk size in characters
    pub max_chunk_size_chars: usize,
    /// Overlap between consecutive chunks in characters
    pub chunk_overlap_chars: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chunk_size_chars: 4000,
            chunk_overlap_chars: 200,
        }
    }
}

/// Split text into chunks respecting max size and overlap
pub fn chunk_text(text: &str, config: &ChunkConfig) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    if text.len() <= config.max_chunk_size_chars {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < text.len() {
        let end = std::cmp::min(start + config.max_chunk_size_chars, text.len());

        // Try to find a natural break point (paragraph, sentence, or word boundary)
        let actual_end = if end < text.len() {
            find_break_point(text, start, end)
        } else {
            end
        };

        let chunk = text[start..actual_end].trim().to_string();
        if !chunk.is_empty() {
            chunks.push(chunk);
        }

        // Move start forward, accounting for overlap
        start = if actual_end >= text.len() {
            text.len()
        } else {
            actual_end.saturating_sub(config.chunk_overlap_chars)
        };

        // Prevent infinite loop
        if start
            <= chunks
                .last()
                .map(|c| text.find(c).unwrap_or(0))
                .unwrap_or(0)
        {
            start = actual_end;
        }
    }

    chunks
}

/// Find a natural break point in the text
fn find_break_point(text: &str, start: usize, end: usize) -> usize {
    let slice = &text[start..end];

    // Try to break at paragraph boundary (double newline)
    if let Some(pos) = slice.rfind("\n\n") {
        return start + pos + 2;
    }

    // Try to break at sentence boundary (. followed by space)
    if let Some(pos) = slice.rfind(". ") {
        return start + pos + 2;
    }

    // Try to break at word boundary
    if let Some(pos) = slice.rfind(' ') {
        return start + pos + 1;
    }

    // Fall back to max chunk size
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let chunks = chunk_text("", &ChunkConfig::default());
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_short_text() {
        let text = "Hello world";
        let chunks = chunk_text(text, &ChunkConfig::default());
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_long_text() {
        let text = "a".repeat(10000);
        let config = ChunkConfig {
            max_chunk_size_chars: 4000,
            chunk_overlap_chars: 200,
        };
        let chunks = chunk_text(&text, &config);
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= 4000);
        }
    }

    #[test]
    fn test_paragraph_break() {
        let text =
            "First paragraph.\n\nSecond paragraph that is much longer and continues for a while.";
        let config = ChunkConfig {
            max_chunk_size_chars: 30,
            chunk_overlap_chars: 5,
        };
        let chunks = chunk_text(text, &config);
        assert!(chunks.len() > 1);
    }
}
