# Tests Module — Phase 2.5: Import, Export, and Desktop Tests

## Module Overview

Phase 2.5 adds tests for the content pipeline (import and export) and the Tauri desktop application. Import tests verify text extraction from multiple formats. Export tests verify Typst compilation produces valid PDFs. Desktop tests verify Tauri commands and file access permissions.

## Phase 2.5 Test Scope

| Test Category | Purpose | Coverage Target |
|---------------|---------|-----------------|
| PDF import tests | Text extraction from text-based and scanned PDFs | 3+ PDF fixtures |
| Markdown import tests | Chunking with heading structure preservation | All heading depths |
| Website import tests | Content extraction, URL validation, timeout | Happy + error paths |
| Chunking tests | Size limits, overlap, truncation | All edge cases |
| Typst export tests | Rendering to Typst, compilation to PDF | Chapter + curriculum |
| Typst error tests | Compile errors returned as structured diagnostics | Common error types |
| Desktop command tests | Tauri commands for import, export, file access | All commands |
| Desktop CSP tests | Content Security Policy enforcement | All CSP directives |

## File Structure

```
tests/
├── import/
│   ├── mod.rs
│   ├── pdf_import_test.rs
│   ├── markdown_import_test.rs
│   ├── text_import_test.rs
│   ├── website_import_test.rs
│   ├── chunker_test.rs
│   ├── deduplication_test.rs
│   └── fixtures/
│       ├── sample-text.pdf
│       ├── sample-markdown.md
│       ├── sample-text.txt
│       ├── sample-latin1.txt
│       ├── sample-broken.pdf        # PDF with no extractable text
│       ├── sample-scanned.pdf       # Image-based PDF (OCR test)
│       └── sample-website.html
├── export/
│   ├── mod.rs
│   ├── typst_render_test.rs
│   ├── typst_compile_test.rs
│   ├── typst_error_test.rs
│   ├── artifact_test.rs
│   └── fixtures/
│       ├── chapter-input.json
│       ├── curriculum-input.json
│       ├── expected-chapter.typst
│       └── invalid-typst.typst
├── citation/
│   ├── mod.rs
│   └── citation_tracking_test.rs
└── desktop/
    ├── mod.rs
    ├── import_command_test.rs
    ├── export_command_test.rs
    └── file_permission_test.rs
```

## Import Tests

### PDF Import Tests

```rust
// import/pdf_import_test.rs (conceptual)
#[tokio::test]
async fn test_import_text_based_pdf_extracts_text() {
    let path = "tests/import/fixtures/sample-text.pdf";
    let result = content_pipeline.import_pdf(path, &default_config()).await.unwrap();

    assert!(!result.chunks.is_empty());
    assert!(result.metadata.word_count > 0);
    assert!(result.metadata.extraction_confidence > 0.8);
}

#[tokio::test]
async fn test_import_pdf_with_no_text_returns_low_confidence() {
    // PDF that's all images, no text layer
    let path = "tests/import/fixtures/sample-broken.pdf";
    let result = content_pipeline.import_pdf(path, &default_config()).await;

    match result {
        Ok(doc) => {
            assert!(doc.metadata.extraction_confidence < 0.5);
            assert!(doc.metadata.warnings.iter().any(|w| w.contains("low confidence")));
        }
        Err(ImportError::ExtractionFailed { .. }) => {
            // Acceptable: extraction completely failed
        }
    }
}

#[tokio::test]
async fn test_import_pdf_with_ocr_fallback() {
    // Scanned PDF → OCR via sandbox → extracted text
    let path = "tests/import/fixtures/sample-scanned.pdf";
    let config = ImportConfig { ocr_enabled: true, ..default() };

    let result = content_pipeline.import_pdf(path, &config).await.unwrap();

    assert!(result.metadata.ocr_applied);
    // OCR will have lower confidence but should produce some text
    assert!(result.metadata.word_count > 0);
}

#[tokio::test]
async fn test_import_nonexistent_pdf_returns_error() {
    let result = content_pipeline.import_pdf(
        Path::new("nonexistent.pdf"),
        &default_config()
    ).await;

    assert!(result.is_err());
}
```

### Markdown Import Tests

```rust
// import/markdown_import_test.rs (conceptual)
#[tokio::test]
async fn test_import_markdown_preserves_heading_structure() {
    let md = r#"
# Chapter 1
## Section 1.1
Content here.
## Section 1.2
More content.
# Chapter 2
Final content.
"#;

    let doc = content_pipeline.import_markdown_str(md, "test.md").await.unwrap();

    assert_eq!(doc.chunks.len(), 4); // Or how the chunker splits it
    // Heading paths should be preserved
    let section_chunk = doc.chunks.iter().find(|c| c.content.contains("Content here"));
    assert!(section_chunk.is_some());
    assert!(section_chunk.unwrap().heading_path.contains(&"Section 1.1".into()));
}

#[tokio::test]
async fn test_import_markdown_code_blocks_preserved() {
    let md = r#"
# Example
```python
def hello():
    print("world")
```
"#;
    let doc = content_pipeline.import_markdown_str(md, "test.md").await.unwrap();
    assert!(doc.chunks[0].content.contains("```python"));
    assert!(doc.chunks[0].content.contains("hello()"));
}
```

### Website Import Tests

```rust
// import/website_import_test.rs (conceptual)
#[tokio::test]
async fn test_import_website_extracts_main_content() {
    // Provide a static HTML fixture (not a real URL fetch in tests)
    let html = r#"
<html>
<body>
  <nav>Navigation menu</nav>
  <article>
    <h1>Python Tutorial</h1>
    <p>Python is a programming language...</p>
    <p>It is widely used for data science...</p>
  </article>
  <footer>Copyright 2024</footer>
</body>
</html>
"#;

    // Mock the HTTP response with this HTML
    let doc = content_pipeline.import_html(html, "https://example.com/tutorial").await.unwrap();

    // Navigation and footer should be stripped
    assert!(!doc.chunks.iter().any(|c| c.content.contains("Navigation menu")));
    assert!(!doc.chunks.iter().any(|c| c.content.contains("Copyright")));

    // Main content should be present
    assert!(doc.chunks.iter().any(|c| c.content.contains("Python Tutorial")));
    assert!(doc.chunks.iter().any(|c| c.content.contains("data science")));
    assert_eq!(doc.origin, "https://example.com/tutorial");
}

#[tokio::test]
async fn test_import_website_rejects_internal_urls() {
    for url in &[
        "http://localhost:8080/page",
        "http://127.0.0.1/admin",
        "http://192.168.1.1/config",
        "file:///etc/passwd",
    ] {
        let result = content_pipeline.import_website(url, &default_config()).await;
        assert!(result.is_err(), "Should reject URL: {}", url);
    }
}

#[tokio::test]
async fn test_import_website_timeout_handled() {
    // Mock server that sleeps for 60 seconds
    // Request with 5s timeout
    // Expect timeout error, not hang
}
```

### Chunking Tests

```rust
// import/chunker_test.rs (conceptual)
#[test]
fn test_chunk_respects_max_size() {
    let text = "A".repeat(10_000);
    let chunks = chunker.chunk(&text, &ChunkConfig {
        max_chunk_size_chars: 4000,
        chunk_overlap_chars: 200,
        ..default()
    });

    for chunk in &chunks {
        assert!(chunk.content.len() <= 4000);
    }
    // 10000 chars with 4000 max + 200 overlap → 3 chunks
    assert_eq!(chunks.len(), 3);
}

#[test]
fn test_chunk_overlap_is_correct() {
    let text: Vec<String> = (0..100).map(|i| format!("Sentence {}.\n", i)).collect();
    let text = text.join("");

    let chunks = chunker.chunk(&text, &ChunkConfig {
        max_chunk_size_chars: 500,
        chunk_overlap_chars: 100,
        ..default()
    });

    // Verify overlap: last N chars of chunk i appear at start of chunk i+1
    for i in 0..chunks.len() - 1 {
        let current_end = &chunks[i].content[chunks[i].content.len().saturating_sub(100)..];
        let next_start = &chunks[i + 1].content[..100.min(chunks[i + 1].content.len())];
        // There should be some overlap
        let overlap_count = current_end.chars().zip(next_start.chars())
            .filter(|(a, b)| a == b)
            .count();
        assert!(overlap_count > 0, "No overlap between chunk {} and {}", i, i + 1);
    }
}

#[test]
fn test_empty_input_produces_no_chunks() {
    let chunks = chunker.chunk("", &default_config());
    assert!(chunks.is_empty());
}

#[test]
fn test_single_sentence_shorter_than_max_is_one_chunk() {
    let chunks = chunker.chunk("Hello world.", &default_config());
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Hello world.");
}
```

### Deduplication Tests

```rust
#[test]
fn test_same_content_produces_same_checksum() {
    let checksum1 = content_pipeline.compute_checksum("Hello world".as_bytes());
    let checksum2 = content_pipeline.compute_checksum("Hello world".as_bytes());
    assert_eq!(checksum1, checksum2);
}

#[test]
fn test_different_content_produces_different_checksum() {
    let checksum1 = content_pipeline.compute_checksum("Hello world".as_bytes());
    let checksum2 = content_pipeline.compute_checksum("Hello World".as_bytes());
    assert_ne!(checksum1, checksum2);
}
```

## Export Tests

### Typst Render Tests

```rust
// export/typst_render_test.rs (conceptual)
#[test]
fn test_chapter_renders_to_valid_typst() {
    let chapter = serde_json::from_str::<Chapter>(include_str!("fixtures/chapter-input.json")).unwrap();
    let typst = typst_renderer.render_chapter(&chapter).unwrap();

    // Basic Typst structure checks
    assert!(typst.contains("= ")); // Title uses = heading
    assert!(typst.contains("*Learning objectives*"));
}

#[test]
fn test_curriculum_renders_with_toc() {
    let curriculum = serde_json::from_str::<CurriculumPlan>(
        include_str!("fixtures/curriculum-input.json")
    ).unwrap();

    let typst = typst_renderer.render_curriculum(&curriculum).unwrap();

    assert!(typst.contains("#table_of_contents()"));
    assert!(typst.contains("#pagebreak()"));
}

#[test]
fn test_rendered_typst_matches_expected() {
    // Snapshot test: render a known input → compare to expected Typst output
    let chapter = load_fixture("chapter-input.json");
    let actual = typst_renderer.render_chapter(&chapter).unwrap();
    let expected = include_str!("fixtures/expected-chapter.typst");

    assert_eq!(actual.trim(), expected.trim());
}
```

### Typst Compile Tests

```rust
// export/typst_compile_test.rs (conceptual)
#[tokio::test]
#[ignore = "requires Docker + Typst sandbox"]
async fn test_compile_valid_typst_produces_pdf() {
    let typst_source = "= Hello\n\nThis is a test document.";
    let result = typst_compiler.compile_to_pdf(&typst_source, &empty_assets(), &sandbox).await.unwrap();

    assert!(!result.data.is_empty());
    // PDF files start with %PDF
    assert!(result.data.starts_with(b"%PDF"));
    assert!(result.size_bytes > 0);
    assert!(!result.checksum.is_empty());
}

#[tokio::test]
#[ignore = "requires Docker + Typst sandbox"]
async fn test_compile_invalid_typst_returns_diagnostics() {
    let typst_source = "= Hello\n\n#this_is_not_valid_typst_syntax[[[";

    let result = typst_compiler.compile_to_pdf(&typst_source, &empty_assets(), &sandbox).await;

    match result {
        Err(ExportError::CompilationFailed { diagnostics }) => {
            assert!(!diagnostics.is_empty());
            assert!(diagnostics.iter().any(|d| d.message.contains("unexpected")));
        }
        other => panic!("Expected CompilationFailed, got {:?}", other),
    }
}
```

### Artifact Tests

```rust
#[test]
fn test_artifact_checksum_is_deterministic() {
    let pdf_data = b"%PDF-1.4 fake pdf content";
    let artifact1 = create_artifact(pdf_data);
    let artifact2 = create_artifact(pdf_data);
    assert_eq!(artifact1.checksum, artifact2.checksum);
}

#[test]
fn test_artifact_records_source_content_ids() {
    // Verify artifact.source_content_ids references the correct chapters/curricula
}
```

## Desktop Tests

```rust
// desktop/import_command_test.rs (conceptual)
#[tokio::test]
#[ignore = "requires Tauri runtime or mock"]
async fn test_import_file_command_opens_dialog() {
    // Test the Tauri command — mocked because native dialogs can't be automated
}

#[tokio::test]
#[ignore = "requires Tauri runtime or mock"]
async fn test_export_command_saves_to_chosen_path() {
    // Test export Tauri command
}
```

## Cross-Phase Integration Scenarios

These tests verify Phase 2.5 components work correctly with Phase 1 and 2 infrastructure.

### Scenario: Import → Ground Lesson → Export

```rust
#[tokio::test]
#[ignore = "requires Phase 2 + 2.5 infrastructure"]
async fn test_import_pdf_ground_lesson_and_export_pdf() {
    // ── Setup ──
    let gateway = MockLlmGateway::start().await;
    let agent = TestAgentCore::start(phase25_config(gateway.url())).await;
    let sandbox = TestSandbox::start().await;  // Docker required

    // ── Step 1: Import a PDF textbook chapter ──
    let source_doc = agent.import_file("tests/fixtures/sample-text.pdf").await.unwrap();
    assert_eq!(source_doc.source_type, SourceType::Pdf);
    assert!(source_doc.chunks.len() >= 3, "PDF should produce multiple chunks");
    assert!(source_doc.metadata.word_count > 100);

    // ── Step 2: Ground a lesson in the imported source ──
    // LLM generates lesson content that cites source chunks
    gateway.expect_completion()
        .respond_with(grounded_lesson_fixture(&source_doc));

    let lesson = agent.generate_grounded_lesson(&source_doc.id, "Explain the key concepts").await.unwrap();

    // Verify citations reference valid source chunks
    for citation in &lesson.citations {
        assert!(source_doc.chunks.iter().any(|c| c.id == citation.source_chunk_id),
            "Citation references non-existent chunk {}", citation.source_chunk_id);
    }

    // ── Step 3: Export the lesson as PDF ──
    let pdf = agent.export_lesson_pdf(&lesson.id).await.unwrap();

    assert!(pdf.data.starts_with(b"%PDF"));
    assert!(pdf.size_bytes > 1000);
    assert!(pdf.page_count.unwrap() >= 1);

    // ── Step 4: Verify provenance chain ──
    // exported PDF → cites lesson → cites source chunks → imported PDF
    assert!(pdf.source_content_ids.contains(&lesson.id));
    for citation in &lesson.citations {
        assert!(citation.source_chunk_id.to_string().len() > 0);
    }
}

#[tokio::test]
#[ignore = "requires Phase 2 + 2.5 infrastructure"]
async fn test_full_content_pipeline_roundtrip() {
    // Import → chunk → store → retrieve → ground → export → verify
    let agent = TestAgentCore::start(phase25_config()).await;

    // 1. Import Markdown document
    let md_content = r#"# Introduction to Algorithms
## What is an Algorithm?
An algorithm is a step-by-step procedure for solving a problem.
## Why Study Algorithms?
Understanding algorithms helps you write efficient code.
### Time Complexity
Time complexity measures how runtime grows with input size.
"#;

    let source = agent.import_markdown_str(md_content, "algorithms.md").await.unwrap();

    // 2. Verify chunking preserved heading structure
    assert_eq!(source.chunks.len(), 3);
    assert_eq!(source.chunks[0].heading_path, vec!["Introduction to Algorithms", "What is an Algorithm?"]);
    assert_eq!(source.chunks[1].heading_path, vec!["Introduction to Algorithms", "Why Study Algorithms?"]);
    assert_eq!(source.chunks[2].heading_path, vec!["Introduction to Algorithms", "Time Complexity"]);

    // 3. Verify chunks have overlap
    let chunk1_end = &source.chunks[0].content[source.chunks[0].content.len().saturating_sub(50)..];
    let chunk2_start = &source.chunks[1].content[..50.min(source.chunks[1].content.len())];
    let overlap = chunk1_end.chars().zip(chunk2_start.chars()).filter(|(a, b)| a == b).count();
    assert!(overlap > 0, "Adjacent chunks should have overlapping text");

    // 4. Export source as Typst → PDF
    let typst = agent.render_source_to_typst(&source.id).await.unwrap();
    assert!(typst.contains("= Introduction to Algorithms"));
    assert!(typst.contains("== What is an Algorithm?"));

    let pdf = agent.compile_typst_to_pdf(&typst).await.unwrap();
    assert!(pdf.data.starts_with(b"%PDF"));

    // 5. Verify artifact reproducibility
    let pdf2 = agent.compile_typst_to_pdf(&typst).await.unwrap();
    assert_eq!(pdf.checksum, pdf2.checksum, "Same Typst source should produce identical PDF");
}

#[tokio::test]
#[ignore = "requires Phase 2.5 infrastructure"]
async fn test_import_pipeline_error_recovery() {
    let agent = TestAgentCore::start(phase25_config()).await;

    // Test 1: Corrupt PDF
    let result = agent.import_file("tests/fixtures/sample-corrupt.pdf").await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("extraction") || err.to_string().contains("failed"));

    // Test 2: Unsupported file type
    let result = agent.import_file("tests/fixtures/image.png").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported"));

    // Test 3: URL with redirect to private IP
    // Mock: initial URL returns 302 to http://192.168.1.1/admin
    // Expected: error after following redirect and detecting private IP
    let result = agent.import_website("https://evil.com/redirect").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(err.contains("blocked") || err.contains("private") || err.contains("internal"));

    // Test 4: Session isolation — imported documents not shared across sessions
    let session_a = agent.create_session().await;
    let session_b = agent.create_session().await;

    let doc = agent.import_markdown_for_session("secret.md", "Confidential content", &session_a.id).await.unwrap();
    let result = agent.get_source_document_for_session(&doc.id, &session_b.id).await;
    assert!(result.is_err() || result.unwrap().is_none(),
        "Session B should not access Session A's imported document");
}
```

## Quality Gates

- [ ] PDF text extraction works on text-based PDFs
- [ ] OCR fallback works for scanned PDFs (low confidence but produces text)
- [ ] Markdown heading structure preserved in chunks
- [ ] Website import strips navigation and extracts main content
- [ ] Website import rejects internal/private URLs
- [ ] Chunks respect max size and overlap correctly
- [ ] Typst rendering produces valid Typst markup
- [ ] Typst compilation produces valid PDF
- [ ] Typst errors return structured diagnostics, not raw stderr
- [ ] Same content imported twice = same checksum (deduplication works)
- [ ] Citations track source chunks correctly
- [ ] All tests use fixture files — no real URLs fetched or real user documents
