use content_pipeline::import::chunker::{chunk_text, ChunkConfig};

#[test]
fn test_empty_input() {
    let chunks = chunk_text("", &ChunkConfig::default());
    assert!(chunks.is_empty());
}

#[test]
fn test_short_text_single_chunk() {
    let text = "Hello world, this is a short text.";
    let chunks = chunk_text(text, &ChunkConfig::default());
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], text);
}

#[test]
fn test_long_text_multiple_chunks() {
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
fn test_paragraph_break_priority() {
    let text = "First paragraph content here.\n\nSecond paragraph content here that continues.";
    let config = ChunkConfig {
        max_chunk_size_chars: 30,
        chunk_overlap_chars: 5,
    };
    let chunks = chunk_text(text, &config);
    assert!(chunks.len() > 1);
}

#[test]
fn test_sentence_break_fallback() {
    let text = "First sentence. Second sentence. Third sentence that is quite long.";
    let config = ChunkConfig {
        max_chunk_size_chars: 25,
        chunk_overlap_chars: 5,
    };
    let chunks = chunk_text(text, &config);
    assert!(chunks.len() > 1);
}

#[test]
fn test_word_break_fallback() {
    let text = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10";
    let config = ChunkConfig {
        max_chunk_size_chars: 20,
        chunk_overlap_chars: 5,
    };
    let chunks = chunk_text(text, &config);
    assert!(chunks.len() > 1);
}

#[test]
fn test_custom_config() {
    let text = "a".repeat(5000);
    let config = ChunkConfig {
        max_chunk_size_chars: 1000,
        chunk_overlap_chars: 100,
    };
    let chunks = chunk_text(&text, &config);
    assert!(chunks.len() >= 5);
}

#[test]
fn test_whitespace_handling() {
    let text = "  Hello   world  \n\n  Test  ";
    let chunks = chunk_text(text, &ChunkConfig::default());
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], "Hello   world  \n\n  Test");
}
