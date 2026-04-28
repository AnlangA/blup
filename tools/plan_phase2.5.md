# Tools Module — Phase 2.5: Typst Export and Content Importer

## Module Overview

Phase 2.5 adds two tools that bridge structured learning content and external documents: `typst-export` converts chapters and curricula to PDF via Typst compilation, and `content-importer` ingests external source materials (PDF, Markdown, text, websites) into structured `SourceDocument` JSON.

Both tools can be used standalone (CLI) or called programmatically by the `content-pipeline` crate. Heavier operations (Typst compilation, website fetching) delegate to Docker sandboxes.

## Deliverables

| Tool | Language | Purpose | Sandbox Required | Status |
|------|----------|---------|-----------------|--------|
| `tools/typst-export/` | Rust | Render learning content to Typst, compile to PDF via sandbox | Yes (compile) | Planned |
| `tools/content-importer/` | Rust | Extract `SourceDocument` from PDF, Markdown, text, and websites | Yes (URL fetch, OCR) | Planned |

## Tool: typst-export

### CLI Specification

```
typst-export 0.1.0
Export learning content to Typst and PDF

USAGE:
    typst-export <COMMAND>

COMMANDS:
    export chapter <chapter-json>       Export a single chapter to PDF
    export curriculum <curriculum-json> Export full curriculum to PDF
    render <input-json>                 Render to Typst without compiling
    render --format typst               Output raw Typst source
    compile <typst-file>                Compile Typst to PDF (via Docker sandbox)
    validate <typst-file>               Validate Typst syntax
    list-templates                      List available templates

FLAGS:
    --output <path>             Output file path (default: ./output.pdf)
    --template <name>           Template to use: chapter, curriculum, flashcards
    --templates-dir <path>      Path to Typst templates directory
    --sandbox                   Use Docker sandbox for compilation (default: true)
    --no-sandbox                Compile on host (dev only — requires typst CLI)
    --fonts-dir <path>          Additional fonts directory
    --title-page                Include title page (curriculum only)
    --toc                       Include table of contents (curriculum only)
    --json                      Output result as JSON (includes checksum, page count)
```

### Detailed Data Flow

```
Chapter/Curriculum JSON (validated against schema)
  │
  ▼
┌─────────────────────────────┐
│ Step 1: Load & Validate      │
│ - Parse input JSON           │
│ - Validate against schema    │
│ - Check for missing assets   │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ Step 2: Render Typst         │
│ - Select template            │
│ - Map JSON fields → Typst    │
│ - Inject fonts/assets refs   │
│ - Generate TOC if needed     │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ Step 3: Validate Typst       │
│ - Check Typst syntax         │
│ - Verify font references     │
│ - Check image paths          │
│ - Warn on missing citations  │
└─────────────┬───────────────┘
              │
              ▼ (if --no-sandbox, skip to Step 5)
┌─────────────────────────────┐
│ Step 4: Compile (Sandbox)    │
│ - Copy Typst + assets to     │
│   sandbox container          │
│ - Run `typst compile`        │
│ - Capture PDF + diagnostics  │
│ - Destroy container          │
└─────────────┬───────────────┘
              │
              ▼
┌─────────────────────────────┐
│ Step 5: Verify Artifact      │
│ - Check PDF is valid (%PDF)  │
│ - Compute SHA-256 checksum   │
│ - Count pages                │
│ - Record provenance          │
└─────────────┬───────────────┘
              │
              ▼
         PDF Artifact
```

### Typst Templates

#### Chapter Template (Full)

```typst
// templates/chapter.typst
// Receives chapter JSON data rendered as a Typst dictionary

#let data = json("/workspace/input.json")

// Page setup
#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2cm),
  header: align(right)[
    #text(size: 9pt, fill: luma(150))[
      #data.at("title")
    ]
  ],
  footer: context [
    #text(size: 9pt, fill: luma(150))[
      Page #counter(page).display()
    ]
  ],
)

#set text(font: ("Inter", "Noto Sans"), size: 11pt, lang: "en")
#set par(justify: true, leading: 0.6em)
#set heading(numbering: "1.")

// Show math in corrected font
#show math.equation: set text(font: ("New Computer Modern Math", "Latin Modern Math"))

// --- Chapter Title ---
#align(center)[
  = #data.at("title")
]

#if "estimated_minutes" in data [
  #align(center)[
    #text(size: 10pt, fill: luma(150))[
      Estimated time: #data.at("estimated_minutes") minutes
    ]
  ]
]

// --- Learning Objectives ---
#if "objectives" in data and data.at("objectives").len() > 0 [
  == Learning Objectives
  #for obj in data.at("objectives") [
    - #obj
  ]
]

// --- Prerequisites ---
#if "prerequisites" in data and data.at("prerequisites").len() > 0 [
  == Prerequisites
  #for pre in data.at("prerequisites") [
    - #pre
  ]
]

#v(1em)

// --- Chapter Content (Markdown rendered to Typst) ---
// The content field is pre-rendered from Markdown to Typst markup
// by the Rust renderer before being passed to the Typst compiler.
// This avoids needing a Markdown parser inside Typst.

#if "content" in data [
  #data.at("content")
]

// --- Key Concepts ---
#if "key_concepts" in data and data.at("key_concepts").len() > 0 [
  == Key Concepts
  #for concept in data.at("key_concepts") [
    - #concept
  ]
]

// --- Exercises ---
#if "exercises" in data and data.at("exercises").len() > 0 [
  #pagebreak()
  == Exercises
  #for (i, ex) in data.at("exercises").enumerate() [
    #set heading(numbering: none)
    === Exercise #(i + 1)
    #set heading(numbering: "1.")

    #ex.at("question")

    #if ex.at("type") == "multiple_choice" [
      #for (j, opt) in ex.at("options", default: ()).enumerate() [
        #link("")[#text(fill: blue)[#box[#counter(heading).display("A")]]] #opt
      ]
    ]
    #v(0.5em)
  ]
]
```

#### Curriculum Template

```typst
// templates/curriculum.typst
#let data = json("/workspace/input.json")

#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2cm),
  footer: context [
    #text(size: 9pt, fill: luma(150))[Page #counter(page).display()]
  ],
)

#set text(font: ("Inter", "Noto Sans"), size: 11pt)

// --- Title Page ---
#align(center + horizon)[
  #v(4cm)
  #text(size: 24pt, weight: "bold")[#data.at("title")]
  #v(0.5cm)
  #text(size: 14pt, fill: luma(100))[#data.at("description", default: "")]
  #v(1cm)

  #if "estimated_duration" in data [
    #text(size: 11pt)[Estimated duration: #data.at("estimated_duration")]
  ]

  #v(2cm)
  #text(size: 10pt, fill: luma(150))[
    Generated by Blup Learning Platform
  ]
]

#pagebreak()

// --- Table of Contents ---
#outline(
  title: [Table of Contents],
  depth: 2,
)

#pagebreak()

// --- Chapters ---
#for chapter in data.at("chapters") [
  #include "chapter.typst"  // Reuse chapter template
  #pagebreak()
]
```

### Markdown → Typst Rendering

The Rust renderer converts chapter content Markdown to Typst markup before passing to the compiler:

```rust
// renderer.rs (conceptual) — Markdown to Typst translation
use pulldown_cmark::{Parser, Event, Tag, CodeBlockKind};

struct TypstRenderer {
    heading_depth_offset: u8,  // Offset for heading levels within chapter context
}

impl TypstRenderer {
    fn render(&self, markdown: &str) -> String {
        let parser = Parser::new(markdown);
        let mut output = String::new();
        let mut in_paragraph = false;

        for event in parser {
            match event {
                Event::Start(Tag::Heading { level, .. }) => {
                    let typst_level = level as u8 + self.heading_depth_offset;
                    output.push_str(&format!("{} ", "=".repeat(typst_level as usize)));
                }
                Event::End(Tag::Heading { .. }) => {
                    output.push('\n');
                }
                Event::Start(Tag::Paragraph) => {
                    in_paragraph = true;
                }
                Event::End(Tag::Paragraph) => {
                    output.push_str("\n\n");
                    in_paragraph = false;
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    let lang = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        _ => String::new(),
                    };
                    output.push_str(&format!("#show raw: set block(below: 0.5em, above: 0.5em)\n"));
                    if !lang.is_empty() {
                        output.push_str(&format!("#raw(lang: \"{}\", block: true, \"", lang));
                    } else {
                        output.push_str("#raw(block: true, \"");
                    }
                }
                Event::End(Tag::CodeBlock(_)) => {
                    // Close raw block
                    output.push_str("\")");
                }
                Event::Start(Tag::List(Some(_))) => {
                    output.push_str("#enum(\n");
                }
                Event::End(Tag::List(Some(_))) => {
                    output.push_str(")\n");
                }
                Event::Start(Tag::List(None)) => {
                    output.push_str("#list(\n");
                }
                Event::End(Tag::List(None)) => {
                    output.push_str(")\n");
                }
                Event::Start(Tag::Item) => {
                    output.push_str("  [");
                }
                Event::End(Tag::Item) => {
                    output.push_str("],\n");
                }
                Event::Start(Tag::Emphasis) => {
                    output.push_str("_");
                }
                Event::End(Tag::Emphasis) => {
                    output.push_str("_");
                }
                Event::Start(Tag::Strong) => {
                    output.push_str("*");
                }
                Event::End(Tag::Strong) => {
                    output.push_str("*");
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    output.push_str(&format!("#link(\"{}\")[", dest_url));
                }
                Event::End(Tag::Link { .. }) => {
                    output.push_str("]");
                }
                Event::InlineMath(math) => {
                    output.push_str(&format!("${}$", math));
                }
                Event::DisplayMath(math) => {
                    output.push_str(&format!("$ {} $", math));
                }
                Event::Text(text) => {
                    // Escape Typst special characters: #, [, ], =
                    let escaped = text
                        .replace("#", "\\#")
                        .replace("[", "\\[")
                        .replace("]", "\\]");
                    output.push_str(&escaped);
                }
                Event::SoftBreak => {
                    output.push(' ');
                }
                Event::HardBreak => {
                    output.push_str("\\\n");
                }
                _ => {}
            }
        }

        output
    }
}
```

### Typst Compilation in Sandbox

```rust
// compiler.rs (conceptual)
pub async fn compile_typst_in_sandbox(
    typst_source: &str,
    assets: &HashMap<String, Vec<u8>>,
    sandbox: &SandboxManager,
    config: &CompileConfig,
) -> Result<CompiledPdf, CompileError> {
    // 1. Create temporary workspace
    let work_dir = tempfile::tempdir()?;

    // 2. Write Typst source to workspace
    tokio::fs::write(work_dir.path().join("input.typst"), typst_source).await?;

    // 3. Write assets (fonts, images) to workspace
    for (name, data) in assets {
        let path = work_dir.path().join(name);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&path, data).await?;
    }

    // 4. Build sandbox request
    let docker_code = format!(
        "cp -r /workspace-input/* /workspace/ && typst compile input.typst output.pdf 2>&1",
    );

    let request = SandboxRequest {
        tool_kind: ToolKind::TypstCompile,
        code: docker_code,
        assets: collect_workspace_files(&work_dir)?,
        limits: SandboxLimits {
            compile_timeout_secs: config.compile_timeout_secs,
            memory_mb: 1024,
            ..SandboxLimits::default()
        },
    };

    // 5. Execute in sandbox
    let result = sandbox.execute(request).await?;

    match result.status {
        ExecutionStatus::Success => {
            // Extract PDF from sandbox output
            let pdf_data = extract_file_from_result(&result, "output.pdf")?;

            // Validate PDF header
            if !pdf_data.starts_with(b"%PDF") {
                return Err(CompileError::InvalidPdfOutput);
            }

            Ok(CompiledPdf {
                data: pdf_data,
                checksum: sha256(&pdf_data),
                size_bytes: pdf_data.len() as u64,
                page_count: count_pdf_pages(&pdf_data)?,
                typst_log: String::from_utf8_lossy(&result.stderr).to_string(),
            })
        }
        ExecutionStatus::TimeoutCompile => {
            Err(CompileError::CompilationTimeout)
        }
        _ => {
            // Parse Typst error diagnostics from stderr
            let diagnostics = parse_typst_errors(&result.stderr);
            Err(CompileError::CompilationFailed { diagnostics })
        }
    }
}
```

### Typst Error Parsing

Typst compiler errors are parsed into structured diagnostics:

```rust
#[derive(Debug, Serialize)]
struct TypstDiagnostic {
    severity: DiagnosticSeverity,  // Error, Warning
    message: String,
    line: Option<u32>,
    column: Option<u32>,
    source_line: Option<String>,
    hint: Option<String>,
}

fn parse_typst_errors(stderr: &str) -> Vec<TypstDiagnostic> {
    let mut diagnostics = Vec::new();

    // Typst error format:
    // error: expected expression, found `#`
    //   ┌─ input.typ:42:15
    //   │
    // 42│   #bad_syntax[[[
    //   │                ^

    let error_pattern = regex::Regex::new(
        r"(?m)^(error|warning): (.+)$\n^  .+ (\d+):(\d+)$\n(?:.+\n)*?\d+\│ (.+)$"
    ).unwrap();

    for cap in error_pattern.captures_iter(stderr) {
        diagnostics.push(TypstDiagnostic {
            severity: match &cap[1] {
                "error" => DiagnosticSeverity::Error,
                "warning" => DiagnosticSeverity::Warning,
                _ => DiagnosticSeverity::Error,
            },
            message: cap[2].to_string(),
            line: cap[3].parse().ok(),
            column: cap[4].parse().ok(),
            source_line: Some(cap[5].to_string()),
            hint: extract_hint(stderr),
        });
    }

    diagnostics
}
```

## Tool: content-importer

### Detailed Extraction by Source Type

#### PDF Extraction

```rust
// pdf.rs (conceptual)
pub async fn extract_pdf(path: &Path, config: &ImportConfig) -> Result<SourceDocument, ImportError> {
    // Strategy 1: Direct text extraction
    let result = pdf_extract::extract_text(path);

    match result {
        Ok(text) if !text.trim().is_empty() && text_quality_score(&text) > 0.5 => {
            // Good extraction — use directly
            return build_document(text, path, ExtractionMethod::DirectText);
        }
        _ => {
            // Strategy 2: pdftotext in sandbox (better than pdf-extract for some PDFs)
            if config.use_sandbox {
                let sandbox_text = extract_pdf_via_sandbox(path).await?;
                if text_quality_score(&sandbox_text) > 0.5 {
                    return build_document(sandbox_text, path, ExtractionMethod::PdfToText);
                }
            }

            // Strategy 3: OCR via tesseract in sandbox
            if config.ocr_enabled {
                let ocr_text = extract_pdf_via_ocr(path).await?;
                return build_document(ocr_text, path, ExtractionMethod::Ocr);
            }

            // Give up with diagnostic
            return Err(ImportError::ExtractionFailed {
                path: path.to_path_buf(),
                reason: "All extraction methods failed".into(),
                tried: vec![ExtractionMethod::DirectText, ExtractionMethod::PdfToText],
            });
        }
    }
}

/// Score text quality: ratio of printable ASCII/Unicode to total characters.
/// Below 0.5 usually indicates garbled extraction.
fn text_quality_score(text: &str) -> f64 {
    let total = text.chars().count() as f64;
    if total == 0.0 { return 0.0; }
    let valid = text.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || c.is_ascii_punctuation())
        .count() as f64;
    valid / total
}
```

#### Website Extraction with Sandbox

```rust
// website.rs (conceptual)
pub async fn extract_website(url: &str, config: &ImportConfig) -> Result<SourceDocument, ImportError> {
    // 1. Validate URL
    let parsed = url::Url::parse(url).map_err(|_| ImportError::InvalidUrl(url.to_string()))?;

    // 2. Security: Reject internal/private URLs
    let host = parsed.host_str().ok_or(ImportError::InvalidUrl(url.to_string()))?;
    if is_private_host(host) {
        return Err(ImportError::UrlBlocked {
            url: url.to_string(),
            reason: "Internal/private URLs are not allowed".into(),
        });
    }

    // 3. Fetch URL in sandbox container
    let fetch_code = format!(r#"
import sys
import json
try:
    import urllib.request
    req = urllib.request.Request('{}', headers={{'User-Agent': 'Blup-ContentImporter/1.0'}})
    with urllib.request.urlopen(req, timeout={}) as resp:
        content = resp.read().decode('utf-8', errors='replace')
        print(json.dumps({{'status': 'ok', 'content': content, 'final_url': resp.url}}))
except Exception as e:
    print(json.dumps({{'status': 'error', 'message': str(e)}}))
    sys.exit(1)
"#, url, config.timeout_secs);

    let request = SandboxRequest {
        tool_kind: ToolKind::ImportFetch,
        code: fetch_code,
        limits: SandboxLimits {
            compile_timeout_secs: config.timeout_secs,
            network_enabled: true,  // Only for this specific URL
            memory_mb: 256,
            ..SandboxLimits::default()
        },
    };

    let result = sandbox.execute(request).await?;

    if result.status != ExecutionStatus::Success {
        return Err(ImportError::FetchFailed {
            url: url.to_string(),
            reason: result.stderr,
        });
    }

    let fetch_result: FetchResult = serde_json::from_str(&result.stdout)?;

    // 4. Extract main content (readability algorithm)
    let html = &fetch_result.content;
    let document = scraper::Html::parse_document(html);

    // Remove non-content elements
    let non_content_selectors = [
        "script", "style", "nav", "footer", "header",
        ".sidebar", ".navigation", ".menu", ".advertisement",
        ".comments", "#comments",
    ];

    let mut content = String::new();
    let article = document.select(scrp("article")).next()
        .or_else(|| document.select(scrp("main")).next())
        .or_else(|| document.select(scrp("[role='main']")).next());

    if let Some(main_element) = article {
        content = main_element.text().collect::<Vec<_>>().join(" ");
    } else {
        // Fall back to body text minus excluded elements
        let body = document.select(scrp("body")).next()
            .ok_or(ImportError::NoContent(url.to_string()))?;
        content = body.text().collect::<Vec<_>>().join(" ");
    }

    // Clean up whitespace
    content = clean_whitespace(&content);

    if content.len() < 100 {
        return Err(ImportError::ContentTooShort {
            url: url.to_string(),
            length: content.len(),
        });
    }

    // 5. Extract title
    let title = document.select(scrp("title")).next()
        .map(|t| t.inner_html())
        .unwrap_or_else(|| url.to_string());

    // 6. Build SourceDocument
    Ok(SourceDocument {
        id: Uuid::new_v4(),
        source_type: SourceType::Website,
        title,
        origin: fetch_result.final_url.unwrap_or(url.to_string()),
        checksum: sha256(content.as_bytes()),
        language: detect_language(&content),
        extracted_at: chrono::Utc::now(),
        metadata: SourceMetadata {
            word_count: content.split_whitespace().count() as u32,
            extraction_method: "fetch+readability".into(),
            extraction_confidence: if content.len() > 500 { 0.9 } else { 0.5 },
            warnings: vec![],
            ..Default::default()
        },
        chunks: chunk_text(&content, &config.chunk_config),
    })
}

fn is_private_host(host: &str) -> bool {
    // Reject localhost, loopback, private IPs, link-local
    host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host.starts_with("192.168.")
        || host.starts_with("10.")
        || host.starts_with("172.16.")
        || host.starts_with("169.254.")
        || host.starts_with("0.")
}
```

### Error Taxonomy

```rust
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("File not found: {path}")]
    FileNotFound { path: PathBuf },

    #[error("Unsupported file type: {extension}")]
    UnsupportedType { extension: String },

    #[error("PDF extraction failed for {path}: {reason}")]
    ExtractionFailed { path: PathBuf, reason: String, tried: Vec<ExtractionMethod> },

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("URL blocked: {url} — {reason}")]
    UrlBlocked { url: String, reason: String },

    #[error("Website fetch failed: {url} — {reason}")]
    FetchFailed { url: String, reason: String },

    #[error("Content too short ({length} chars) from {url}")]
    ContentTooShort { url: String, length: usize },

    #[error("No content found at {0}")]
    NoContent(String),

    #[error("Encoding detection failed for {path}")]
    EncodingError { path: PathBuf },

    #[error("Chunking error: {0}")]
    ChunkingError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

## Testing Strategy

### typst-export Tests

| Test | Method | Expected |
|------|--------|----------|
| Chapter → Typst rendering | Valid chapter JSON fixture | Typst output contains = heading, objectives list |
| Curriculum → Typst with TOC | Valid curriculum JSON | Output contains `#outline()` and chapter includes |
| Markdown → Typst translation | Markdown with headings, code, math, lists | Typst uses correct syntax for each |
| Typst compile → valid PDF | Docker sandbox with typst | Output starts with `%PDF`, non-empty |
| Typst compile error → diagnostics | Invalid Typst with syntax error | Structured error with line number and message |
| Missing font → warning | Typst with non-existent font | Warning emitted, compilation uses fallback |
| Empty chapter → valid PDF | Chapter with no content | PDF with title page only |
| Template validation | All templates parsed | No syntax errors in `.typst` files |
| Large chapter → no timeout | 100KB Markdown content | Compiles successfully within 60s |

### content-importer Tests

| Test | Method | Expected |
|------|--------|----------|
| PDF text extraction | Known-good text PDF | Correct text content |
| PDF with garbled text → OCR fallback | Image-based PDF | OCR produces text (low confidence) |
| PDF with no content → error | Empty/corrupt PDF | `ExtractionFailed` error with tried methods |
| Markdown heading preservation | MD with 3 heading levels | Chunks have correct heading paths |
| Text encoding detection | Latin-1 encoded file | Content decoded correctly |
| UTF-16 encoding | UTF-16 LE file | Content decoded correctly |
| Website main content extraction | HTML with nav + article | Nav stripped, article content present |
| Website URL validation | `http://localhost/admin` | `UrlBlocked` error |
| Website URL validation | `file:///etc/passwd` | `InvalidUrl` or `UrlBlocked` |
| Website timeout | Mock server with 60s delay | Timeout error after configurable period |
| Chunk size enforcement | 10K character text | All chunks ≤ max size |
| Chunk overlap | Verify adjacent chunks | Overlap text present |
| Deduplication | Same file twice | Same checksum |
| Language detection | English, Chinese, Spanish | Correct ISO 639-1 code |
| Image extraction from Markdown | MD with `![](image.png)` | Image references in metadata |

## Quality Gates

- [ ] `typst-export` produces valid PDF from chapter and curriculum fixtures
- [ ] Typst compilation errors return structured diagnostics (line, column, message)
- [ ] Typst compilation always uses Docker sandbox (unless `--no-sandbox` for dev)
- [ ] All Typst templates are syntactically valid
- [ ] `content-importer` extracts text from PDF, Markdown, text, and HTML fixtures
- [ ] `content-importer` rejects all internal/private URLs
- [ ] Website import always uses Docker sandbox for fetching
- [ ] Chunk overlap is correct (verified by string comparison of adjacent chunks)
- [ ] Encoding detection works for UTF-8, Latin-1, UTF-16 LE/BE
- [ ] All tools exit with clear, structured error messages on failure (not stack traces)
- [ ] Imported documents are deduplicated by checksum
- [ ] Both tools have `--help` with complete usage documentation
