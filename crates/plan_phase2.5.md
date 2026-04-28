# Crates Module — Phase 2.5: Content Pipeline

## Module Overview

`crates/content-pipeline` handles import of source materials (PDF, Markdown, text, websites) and export of learning documents (Typst → PDF). It is the bridge between external content and the structured learning system. It coordinates with `sandboxes/` for Typst compilation and `storage` for document persistence.

## Phase 2.5 Scope

| Deliverable | Description | Status |
|-------------|-------------|--------|
| Content import | Extract structured `SourceDocument` from PDF, text, Markdown, websites | Planned |
| Content export | Convert chapter/curriculum content to Typst, compile to PDF | Planned |
| Source chunking | Split imported documents into manageable chunks with metadata | Planned |
| Citation tracking | Link LLM-generated content back to source chunks | Planned |
| Artifact management | Track generated PDFs/Typst files as artifacts with checksums | Planned |

## File Structure

```
crates/content-pipeline/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs                    # Pipeline configuration
│   ├── import/
│   │   ├── mod.rs
│   │   ├── pdf.rs                  # PDF text extraction (pdf-extract or lopdf)
│   │   ├── markdown.rs             # Markdown parsing and chunking
│   │   ├── text.rs                 # Plain text processing
│   │   ├── website.rs              # Website fetch + content extraction
│   │   ├── chunker.rs              # Text chunking strategies
│   │   └── metadata.rs             # Source metadata extraction
│   ├── export/
│   │   ├── mod.rs
│   │   ├── typst_renderer.rs       # Chapter/curriculum → Typst markup
│   │   ├── typst_compiler.rs       # Typst → PDF via sandbox
│   │   ├── pdf_artifact.rs         # PDF artifact metadata and storage
│   │   └── templates/
│   │       ├── chapter.typst       # Typst template for single chapter
│   │       └── curriculum.typst    # Typst template for full curriculum
│   ├── models/
│   │   ├── mod.rs
│   │   ├── source_document.rs
│   │   ├── source_chunk.rs
│   │   ├── import_job.rs
│   │   ├── export_job.rs
│   │   └── document_artifact.rs
│   ├── citation/
│   │   ├── mod.rs
│   │   └── tracker.rs              # Source chunk → generated content citation
│   └── error.rs
└── tests/
    ├── pdf_import_test.rs
    ├── markdown_import_test.rs
    ├── chunker_test.rs
    ├── typst_render_test.rs
    ├── typst_compile_test.rs
    └── citation_test.rs
```

## Import Pipeline

### Flow

```
User selects file (PDF / Markdown / text)
  ↓
Content Pipeline receives ImportJob
  ↓
Extract text from source
  ├── PDF: pdf-extract crate or call pdftotext in sandbox
  ├── Markdown: parse to AST, extract text + code blocks
  ├── Text: read directly
  └── Website: fetch URL, extract main content (readability algorithm)
  ↓
Validate extraction quality
  ├── Empty result → error with diagnostic
  ├── Garbled text (PDF OCR issue) → warning + metadata flag
  └── Success → continue
  ↓
Compute content hash (SHA-256) for deduplication
  ↓
Chunk document into sections
  ├── By heading (Markdown)
  ├── By paragraph + overlap (text/PDF)
  └── By section (website)
  ↓
Create SourceDocument with metadata
  ↓
Store in database (via storage crate)
  ↓
Return ImportResult
```

### SourceDocument Model

```rust
// models/source_document.rs (conceptual)
pub struct SourceDocument {
    pub id: Uuid,
    pub source_type: SourceType,
    pub title: String,
    pub origin: String,              // File path (local only) or URL
    pub checksum: String,            // SHA-256
    pub language: String,            // "en", "zh", etc.
    pub license_or_usage_note: Option<String>,
    pub extracted_at: DateTime<Utc>,
    pub metadata: SourceMetadata,
    pub chunks: Vec<SourceChunk>,
}

pub enum SourceType {
    Pdf,
    Markdown,
    PlainText,
    Website,
}

pub struct SourceMetadata {
    pub page_count: Option<u32>,          // PDF
    pub word_count: u32,
    pub character_count: u32,
    pub extraction_method: String,        // "pdftotext", "pdf-extract", "fetch+readability"
    pub extraction_confidence: f32,       // 0.0 - 1.0
    pub ocr_applied: bool,
    pub warnings: Vec<String>,
}

pub struct SourceChunk {
    pub id: Uuid,
    pub document_id: Uuid,
    pub index: u32,
    pub content: String,                 // Max 4000 chars
    pub heading_path: Vec<String>,       // ["Chapter 1", "Section 1.1"]
    pub token_count: u32,
    pub overlap_with_previous: bool,
}
```

### PDF Import

```rust
// import/pdf.rs (conceptual)
pub async fn import_pdf(
    file_path: &Path,
    config: &ImportConfig,
) -> Result<SourceDocument, ImportError> {
    // 1. Validate file exists and is readable
    // 2. Extract text using pdf-extract or lopdf crate
    // 3. If text extraction yields mostly garbage, try OCR via sandbox
    //    (tesseract in a Docker container)
    // 4. If still poor quality, return with low confidence + warning
    // 5. Compute SHA-256 checksum
    // 6. Chunk by page + paragraph
    // 7. Return SourceDocument

    // For Phase 2.5, prefer pdf-extract crate.
    // Only use OCR sandbox as fallback when text extraction confidence < 0.5.
}
```

### Website Import

```rust
// import/website.rs (conceptual)
pub async fn import_website(
    url: &str,
    config: &ImportConfig,
) -> Result<SourceDocument, ImportError> {
    // 1. Validate URL (must be http/https; no file:// or internal IPs)
    // 2. Fetch URL with timeout (10s)
    // 3. Extract main content:
    //    - Use readability algorithm (port of Mozilla Readability)
    //    - Strip navigation, ads, sidebars, footers
    //    - Preserve headings, paragraphs, code blocks, images (alt text)
    // 4. Record access metadata: URL, title, access_time, content_hash
    // 5. Add usage notes: "Fetched from {url} on {date}"
    // 6. Chunk by heading structure
    // 7. Return SourceDocument

    // Security: URL must not point to localhost, private IPs, or file://
    // The actual fetch goes through a sandbox with restricted network.
}
```

### Chunking Strategy

```rust
// import/chunker.rs (conceptual)
pub struct ChunkConfig {
    pub max_chunk_size_chars: usize,    // 4000
    pub chunk_overlap_chars: usize,     // 200
    pub prefer_split_at: Vec<SplitPoint>,
}

pub enum SplitPoint {
    Heading,        // ## or ###
    Paragraph,      // double newline
    Sentence,       // period + space
    Word,           // space (fallback)
}

pub fn chunk_document(
    content: &str,
    structure: &DocumentStructure,
    config: &ChunkConfig,
) -> Vec<SourceChunk> {
    // 1. Identify natural split points (headings first, then paragraphs)
    // 2. Split into chunks that respect max_chunk_size_chars
    // 3. Add overlap between consecutive chunks (chunk_overlap_chars)
    // 4. Preserve heading path for each chunk
    // 5. Estimate token count (approx chars / 4)
}
```

## Export Pipeline

### Flow

```
User requests export (chapter or full curriculum)
  ↓
Content Pipeline receives ExportJob
  ↓
Retrieve content from storage
  ↓
Render to Typst markup
  ├── Chapter template: title, content, exercises, key concepts
  └── Curriculum template: title page, TOC, all chapters
  ↓
Validate Typst syntax (basic checks)
  ↓
Send Typst source to sandbox for compilation
  ├── typst compile → PDF
  └── Capture compile errors as structured diagnostics
  ↓
Compute artifact checksum
  ↓
Store artifact metadata
  ↓
Return DocumentArtifact (PDF binary + metadata)
```

### Typst Templates

#### Chapter Template (`templates/chapter.typst`)

```typst
// Chapter template — receives JSON data at render time
#let chapter = json("chapter.json")

= #chapter.title

*Learning objectives:* #chapter.objectives.join(", ")

#chapter.content

#if chapter.exercises.len() > 0 [
  == Exercises
  #for (i, ex) in chapter.exercises.enumerate() [
    *Exercise #(i + 1):* #ex.question
  ]
]

== Key Concepts
#for concept in chapter.key_concepts [
  - #concept
]
```

#### Curriculum Template (`templates/curriculum.typst`)

```typst
// Full curriculum template
#let curriculum = json("curriculum.json")

#align(center)[
  = #curriculum.title
  #curriculum.description
]

#pagebreak()
#table_of_contents()
#pagebreak()

#for chapter in curriculum.chapters [
  #include "chapter.typst"  // or inline chapter content
  #pagebreak()
]
```

### Typst Compilation (Full Implementation)

```rust
// export/typst_compiler.rs
use std::io::Cursor;

pub async fn compile_to_pdf(
    typst_source: &str,
    assets: &HashMap<String, Vec<u8>>,
    sandbox: &SandboxManager,
) -> Result<DocumentArtifact, ExportError> {
    // 1. Build sandbox command: write assets, compile, output PDF
    let mut setup_commands = String::new();
    for (name, data) in assets {
        let encoded = base64::encode(data);
        setup_commands.push_str(&format!(
            "echo '{}' | base64 -d > /workspace/{} && ", encoded, name
        ));
    }

    let command = format!(
        "{} echo '{}' | base64 -d > /workspace/input.typst && \
         typst compile /workspace/input.typst /workspace/output.pdf 2>&1 && \
         cat /workspace/output.pdf | base64",
        setup_commands,
        base64::encode(typst_source.as_bytes()),
    );

    let request = SandboxRequest {
        tool_kind: ToolKind::TypstCompile,
        code: command,
        limits: SandboxLimits {
            compile_timeout_secs: 60,
            memory_mb: 1024,
            ..SandboxLimits::default()
        },
    };

    let result = sandbox.execute(request).await?;

    match result.status {
        ExecutionStatus::Success => {
            // Decode base64 PDF from stdout
            let pdf_data = base64::decode(&result.stdout.trim())
                .map_err(|e| ExportError::InvalidPdfOutput(e.to_string()))?;

            // Validate PDF header
            if pdf_data.len() < 5 || &pdf_data[..5] != b"%PDF-" {
                return Err(ExportError::InvalidPdfOutput("Output does not start with %PDF-".into()));
            }

            let checksum = format!("sha256:{}", hex::encode(sha2::Sha256::digest(&pdf_data)));
            let page_count = count_pdf_pages(&pdf_data)?;

            Ok(DocumentArtifact {
                id: Uuid::new_v4(),
                session_id: Uuid::nil(), // Set by caller
                format: ArtifactFormat::Pdf,
                data: pdf_data,
                checksum,
                size_bytes: pdf_data.len() as u64,
                page_count: Some(page_count),
                generated_at: Utc::now(),
                source_content_ids: vec![],
                source_typst: typst_source.to_string(),
            })
        }
        ExecutionStatus::TimeoutCompile => {
            Err(ExportError::CompilationTimeout)
        }
        _ => {
            let diagnostics = parse_typst_errors(&result.stderr);
            Err(ExportError::CompilationFailed { diagnostics })
        }
    }
}

/// Count pages in a PDF by counting `/Type /Page` entries (excluding `/Pages`).
fn count_pdf_pages(data: &[u8]) -> Result<u32, ExportError> {
    let text = String::from_utf8_lossy(data);
    // Count page objects: look for "/Type /Page" not followed by "s"
    let re = regex::Regex::new(r"/Type\s*/Page[^s]").unwrap();
    Ok(re.find_iter(&text).count() as u32)
}
```

### Source Format Detection

```rust
// import/detector.rs
#[derive(Debug, PartialEq)]
pub enum DetectedFormat {
    Pdf,
    Markdown,
    PlainText { encoding: &'static str },
    Html,
    Json,
    Unknown { extension: String, mime: Option<String> },
}

pub fn detect_format(path: &Path, first_bytes: &[u8]) -> DetectedFormat {
    // 1. Check magic bytes first (most reliable)
    if first_bytes.starts_with(b"%PDF-") {
        return DetectedFormat::Pdf;
    }
    if first_bytes.starts_with(b"<!DOCTYPE html") || first_bytes.starts_with(b"<html") {
        return DetectedFormat::Html;
    }
    if first_bytes.starts_with(b"{") || first_bytes.starts_with(b"[") {
        // Could be JSON — validate
        if serde_json::from_slice::<serde_json::Value>(first_bytes).is_ok() {
            return DetectedFormat::Json;
        }
    }

    // 2. Check extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        match ext.to_lowercase().as_str() {
            "pdf" => return DetectedFormat::Pdf,
            "md" | "markdown" => return DetectedFormat::Markdown,
            "txt" | "text" => return DetectedFormat::PlainText {
                encoding: detect_encoding(first_bytes)
            },
            "html" | "htm" => return DetectedFormat::Html,
            "json" => return DetectedFormat::Json,
            _ => {}
        }
    }

    // 3. Heuristic: high ratio of printable ASCII → plain text
    let printable_ratio = first_bytes.iter()
        .filter(|b| b.is_ascii_graphic() || b.is_ascii_whitespace())
        .count() as f64 / first_bytes.len().max(1) as f64;

    if printable_ratio > 0.9 {
        return DetectedFormat::PlainText {
            encoding: detect_encoding(first_bytes)
        };
    }

    DetectedFormat::Unknown {
        extension: path.extension().and_then(|e| e.to_str()).unwrap_or("").into(),
        mime: None,
    }
}

fn detect_encoding(data: &[u8]) -> &'static str {
    // Check BOM
    if data.starts_with(&[0xEF, 0xBB, 0xBF]) { return "UTF-8"; }
    if data.starts_with(&[0xFE, 0xFF]) { return "UTF-16BE"; }
    if data.starts_with(&[0xFF, 0xFE]) { return "UTF-16LE"; }

    // Heuristic: if high bytes are common, likely UTF-8; otherwise ASCII/Latin-1
    let non_ascii = data.iter().filter(|b| **b > 127).count();
    if non_ascii > 0 {
        // Try to validate as UTF-8
        if std::str::from_utf8(data).is_ok() {
            return "UTF-8";
        }
        return "LATIN-1"; // Fallback for legacy encodings
    }
    "ASCII"
}
```

### DocumentArtifact Model

```rust
// models/document_artifact.rs (conceptual)
pub struct DocumentArtifact {
    pub id: Uuid,
    pub session_id: Uuid,
    pub format: ArtifactFormat,
    pub data: Vec<u8>,                  // PDF binary
    pub checksum: String,
    pub size_bytes: u64,
    pub generated_at: DateTime<Utc>,
    pub source_content_ids: Vec<Uuid>,  // Source chapters/curricula
    pub source_typst: String,           // Typst source for reproducibility
}

pub enum ArtifactFormat {
    Pdf,
    Typst,
    // Future: Html, Epub
}
```

## Citation Tracking

When LLM-generated learning content uses imported materials, citations must be tracked:

```rust
// citation/tracker.rs (conceptual)
pub struct Citation {
    pub source_chunk_id: Uuid,
    pub target_message_id: Uuid,        // The message that used this source
    pub relevance_score: f32,           // How relevant this chunk was
    pub usage_type: CitationUsageType,
}

pub enum CitationUsageType {
    DirectQuote,
    Paraphrase,
    Background,
    Example,
}

pub struct CitationTracker {
    // Maps: message_id → Vec<Citation>
}
```

Citations are stored in the database and can be surfaced in the UI ("This explanation draws from: 'Introduction to Calculus', Chapter 3").

## Cargo Dependencies

```toml
[dependencies]
# Internal
storage = { path = "../storage" }

# PDF extraction
pdf-extract = "0.7"
lopdf = "0.32"               # Low-level PDF manipulation (fallback)

# Markdown parsing
pulldown-cmark = "0.11"

# HTTP client (website import)
reqwest = { version = "0.12", features = ["json"] }

# HTML parsing (website import readability)
scraper = "0.19"
readability = "0.3"          # Port of Mozilla Readability (or implement simplified version)

# Content hashing
sha2 = "0.10"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Async
tokio = { version = "1", features = ["fs", "process"] }
async-trait = "0.1"

# Error handling
thiserror = "1"

# Logging
tracing = "0.1"

# UUID
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

## Testing Strategy

| Test Category | Method | Scope |
|---------------|--------|-------|
| PDF text extraction | Unit test with fixture PDFs | Known PDF content → expected text |
| PDF OCR fallback | Integration test | Scanned PDF → OCR → text quality check |
| Markdown chunking | Unit test | Markdown with headings → correct chunk boundaries |
| Website extraction | Integration test | Static HTML fixture → extracted content |
| Chunk overlap | Unit test | Verify overlap text between consecutive chunks |
| Typst rendering | Unit test | Chapter JSON → Typst source matches expected |
| Typst compilation | Integration test with Docker | Typst source → PDF output; verify PDF is valid |
| Typst compile error | Integration test | Invalid Typst → structured diagnostics |
| Citation tracking | Unit test | Source chunk → generated content link |
| Deduplication | Unit test | Same file imported twice → same checksum, no duplicate |
| URL validation | Unit test | Reject localhost, private IPs, file:// URLs |
| Artifact checksum | Unit test | Same input → same checksum (deterministic) |

## Quality Gates

- [ ] PDF text extraction works on text-based PDFs (not scanned images)
- [ ] OCR fallback works for scanned PDFs (via sandbox)
- [ ] Chunking respects max size and overlap settings
- [ ] Website import strips navigation and extracts main content
- [ ] Website import rejects internal/private URLs
- [ ] Typst template produces valid Typst markup
- [ ] Typst compilation produces valid PDF
- [ ] Compile errors are returned as structured diagnostics, not raw stderr
- [ ] Imported documents are deduplicated by checksum
- [ ] Citations reference correct source chunks
- [ ] No imported private data in logs
- [ ] All import/export operations respect file permissions

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| PDF text extraction fails on complex layouts | Lost content | Multi-strategy extraction; OCR fallback; warn user on low confidence |
| Typst compilation is slow for large documents | Timeout, poor UX | Chunked compilation for large curricula; progress reporting |
| Website import fetches malicious content | XSS, content injection | Sandboxed fetch; strip HTML; validate extracted text |
| Chunking breaks context across boundaries | Loss of meaning in LLM context | Overlap between chunks; heading path preservation |
| Typst template drift vs schema changes | Broken exports | Contract tests: chapter schema → typst template output is parseable |
