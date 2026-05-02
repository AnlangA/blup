use std::path::Path;
use content_pipeline::import::text::import_text;

#[tokio::test]
async fn test_import_utf8_text() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, "Hello world!\nThis is a test.").unwrap();

    let result = import_text(&file_path).await.unwrap();
    assert_eq!(result.chunks.len(), 1);
    assert!(result.chunks[0].content.contains("Hello world!"));
    assert_eq!(result.metadata.extraction_method.to_string(), "text_read");
}

#[tokio::test]
async fn test_import_nonexistent_text_returns_error() {
    let result = import_text(Path::new("nonexistent.txt")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_import_text_with_encoding_detection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // Write UTF-8 content with BOM
    let content = vec![0xEF, 0xBB, 0xBF]; // UTF-8 BOM
    let mut content = content;
    content.extend_from_slice("Hello world!".as_bytes());
    std::fs::write(&file_path, content).unwrap();

    let result = import_text(&file_path).await.unwrap();
    assert_eq!(result.language, Some("en".to_string()));
}
