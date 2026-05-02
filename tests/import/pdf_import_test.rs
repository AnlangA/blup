use std::path::Path;
use content_pipeline::import::pdf::import_pdf;

#[tokio::test]
async fn test_import_nonexistent_pdf_returns_error() {
    let result = import_pdf(Path::new("nonexistent.pdf")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_import_invalid_file_returns_error() {
    // Create a temporary file with invalid PDF content
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("invalid.pdf");
    std::fs::write(&file_path, "This is not a PDF file").unwrap();

    let result = import_pdf(&file_path).await;
    assert!(result.is_err());
}

// Note: Testing actual PDF extraction requires a valid PDF fixture
// In a real test suite, we would have test fixtures in tests/fixtures/
