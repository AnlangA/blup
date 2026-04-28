# Tools Module — Phase 3: Plugin Builder and Asset Optimizer

## Module Overview

Phase 3 adds two tools: `plugin-builder` packages, validates, and tests domain plugins for distribution, and `asset-optimizer` optimizes images, fonts, audio, and 3D models with recorded provenance.

## Deliverables

| Tool | Language | Purpose | Status |
|------|----------|---------|--------|
| `tools/plugin-builder/` | Rust | Package, validate, and test plugins for distribution | Planned |
| `tools/asset-optimizer/` | Rust | Optimize assets for web and Bevy with provenance tracking | Planned |

## Tool: plugin-builder

### CLI Specification

```
plugin-builder 0.1.0
Package and validate Blup plugins

USAGE:
    plugin-builder <COMMAND>

COMMANDS:
    validate <plugin-dir>      Validate plugin manifest and structure
    test <plugin-dir>          Run plugin contract tests
    build <plugin-dir>         Package plugin for distribution
    inspect <plugin-archive>   Inspect a packaged plugin archive

FLAGS:
    --output <path>            Output archive path
    --format <format>          Archive format: tar.gz (default), zip
    --strict                   Treat warnings as errors
    --json                     Output results as JSON
```

### Contract Test Runner (Full Implementation)

```rust
// test_runner.rs — Runs contract tests against a plugin
use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;
use tokio::time::timeout;

struct ContractTestRunner {
    plugin_dir: PathBuf,
    test_timeout: Duration,          // 30s per test
    plugin_startup_timeout: Duration, // 10s
    test_port: u16,                   // Random available port
}

struct TestCase {
    name: String,
    capability_id: String,
    request: serde_json::Value,
    expected_status: ExpectedStatus,
    expected_output_schema: Option<String>,
}

enum ExpectedStatus {
    Success,
    Error { code_contains: Option<String> },
    PermissionDenied,
}

impl ContractTestRunner {
    async fn run_all(&self, manifest: &PluginManifest) -> Result<TestReport> {
        let mut report = TestReport::new();

        // ── Start plugin process ──
        let mut child = self.start_plugin(manifest)?;
        self.wait_for_health(&mut child).await?;

        // ── Generate test cases from manifest capabilities ──
        let test_cases = self.generate_test_cases(manifest);

        for test_case in test_cases {
            let result = timeout(self.test_timeout, async {
                self.run_single_test(&test_case).await
            }).await;

            match result {
                Ok(Ok(())) => report.passed(test_case.name),
                Ok(Err(e)) => report.failed(test_case.name, e.to_string()),
                Err(_) => report.failed(test_case.name, "Test timed out".into()),
            }
        }

        // ── Stop plugin ──
        self.stop_plugin(&mut child)?;

        report
    }

    fn start_plugin(&self, manifest: &PluginManifest) -> Result<Child> {
        let mut cmd = Command::new("python3");
        cmd.arg(&manifest.runtime.entrypoint)
            .current_dir(&self.plugin_dir)
            .env_clear()
            .env("PORT", self.test_port.to_string())
            .env("PLUGIN_ID", &manifest.plugin_id)
            .env("BLUP_TEST_MODE", "1") // Signal plugin to run in test mode
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to start plugin process: {}", e))?;

        Ok(child)
    }

    async fn wait_for_health(&self, child: &mut Child) -> Result<()> {
        let health_url = format!("http://127.0.0.1:{}/health", self.test_port);
        let start = std::time::Instant::now();

        loop {
            if start.elapsed() > self.plugin_startup_timeout {
                // Collect stderr for diagnostics
                let stderr = child.stderr.as_mut()
                    .map(|s| {
                        let mut buf = String::new();
                        BufReader::new(s).read_line(&mut buf).ok();
                        buf
                    })
                    .unwrap_or_default();
                return Err(anyhow!("Plugin health check timed out after {:?}. Stderr: {}",
                    self.plugin_startup_timeout, stderr));
            }

            if let Ok(resp) = reqwest::get(&health_url).await {
                if resp.status().is_success() {
                    return Ok(());
                }
            }

            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    fn generate_test_cases(&self, manifest: &PluginManifest) -> Vec<TestCase> {
        let mut cases = Vec::new();

        for capability in &manifest.capabilities {
            // Test 1: Valid request → success
            cases.push(TestCase {
                name: format!("{}/valid_request", capability.id),
                capability_id: capability.id.clone(),
                request: self.generate_valid_request(capability),
                expected_status: ExpectedStatus::Success,
                expected_output_schema: Some(capability.output_schema.clone()),
            });

            // Test 2: Malformed request → error
            cases.push(TestCase {
                name: format!("{}/malformed_request", capability.id),
                capability_id: capability.id.clone(),
                request: serde_json::json!({"invalid": "this is not the right shape"}),
                expected_status: ExpectedStatus::Error {
                    code_contains: Some("INVALID_REQUEST".into()),
                },
                expected_output_schema: None,
            });

            // Test 3: Missing required field → error
            cases.push(TestCase {
                name: format!("{}/missing_required_field", capability.id),
                capability_id: capability.id.clone(),
                request: serde_json::json!({}), // Empty request
                expected_status: ExpectedStatus::Error {
                    code_contains: Some("MISSING_REQUIRED_FIELD".into()),
                },
                expected_output_schema: None,
            });
        }

        // Permission tests: for each forbidden permission, verify it's denied
        let forbidden_permissions = [
            "direct:filesystem", "direct:network", "direct:shell",
            "direct:database", "direct:other_plugin",
        ];

        for perm in forbidden_permissions {
            cases.push(TestCase {
                name: format!("permission/deny_{}", perm.replace(':', "_")),
                capability_id: perm.to_string(),
                request: serde_json::json!({"action": "probe"}),
                expected_status: ExpectedStatus::PermissionDenied,
                expected_output_schema: None,
            });
        }

        cases
    }

    fn generate_valid_request(&self, capability: &CapabilityDef) -> serde_json::Value {
        // Generate a minimal but valid request based on the capability's input schema
        let schema_path = self.plugin_dir.join("schemas").join(&capability.input_schema);
        if schema_path.exists() {
            let schema: serde_json::Value = serde_json::from_reader(
                std::fs::File::open(&schema_path).unwrap()
            ).unwrap();
            self.generate_minimal_valid_instance(&schema)
        } else {
            serde_json::json!({"input": "test"})
        }
    }

    fn generate_minimal_valid_instance(&self, schema: &serde_json::Value) -> serde_json::Value {
        // Walk schema properties and provide minimal valid values
        let mut instance = serde_json::Map::new();

        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (name, prop) in props {
                if schema.get("required").and_then(|r| r.as_array())
                    .map_or(false, |r| r.iter().any(|v| v.as_str() == Some(name)))
                {
                    let value = match prop.get("type").and_then(|t| t.as_str()) {
                        Some("string") => {
                            let min_len = prop.get("minLength").and_then(|l| l.as_u64()).unwrap_or(1);
                            serde_json::Value::String("x".repeat(min_len as usize))
                        }
                        Some("integer") | Some("number") => serde_json::json!(1),
                        Some("boolean") => serde_json::json!(true),
                        Some("array") => serde_json::json!([]),
                        Some("object") => serde_json::json!({}),
                        _ => serde_json::Value::Null,
                    };
                    instance.insert(name.clone(), value);
                }
            }
        }

        serde_json::Value::Object(instance)
    }

    async fn run_single_test(&self, test: &TestCase) -> Result<()> {
        let url = format!("http://127.0.0.1:{}/capability/{}",
            self.test_port, test.capability_id);

        let response = reqwest::Client::new()
            .post(&url)
            .json(&test.request)
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;

        match &test.expected_status {
            ExpectedStatus::Success => {
                if !response.status().is_success() {
                    return Err(anyhow!("Expected success but got {}: {}",
                        response.status(), body));
                }
                // Validate output against schema if specified
                if let Some(schema_name) = &test.expected_output_schema {
                    self.validate_against_schema(&body, schema_name)?;
                }
            }
            ExpectedStatus::Error { code_contains } => {
                if response.status().is_success() {
                    return Err(anyhow!("Expected error but got success"));
                }
                if let Some(expected_code) = code_contains {
                    let error_code = body["error"]["code"].as_str().unwrap_or("");
                    if !error_code.contains(expected_code.as_str()) {
                        return Err(anyhow!(
                            "Expected error code containing '{}' but got '{}'",
                            expected_code, error_code
                        ));
                    }
                }
            }
            ExpectedStatus::PermissionDenied => {
                let error_code = body["error"]["code"].as_str().unwrap_or("");
                if !error_code.contains("PERMISSION") && !error_code.contains("FORBIDDEN") {
                    return Err(anyhow!("Expected permission denied but got: {}", body));
                }
            }
        }

        Ok(())
    }

    fn stop_plugin(&self, child: &mut Child) -> Result<()> {
        // Send SIGTERM
        #[cfg(unix)]
        unsafe {
            libc::kill(child.id() as i32, libc::SIGTERM);
        }

        // Wait for graceful shutdown
        let wait_result = child.wait_timeout(Duration::from_secs(5));

        match wait_result {
            Ok(Some(status)) => {
                tracing::info!(exit_code = ?status.code(), "Plugin stopped");
            }
            Ok(None) | Err(_) => {
                // Force kill
                let _ = child.kill();
                let _ = child.wait();
                tracing::warn!("Plugin force-killed after timeout");
            }
        }

        Ok(())
    }
}
```

### Package Builder (Full Implementation)

```rust
// builder.rs — Package a validated plugin into a distributable archive
use std::fs::{self, File};
use std::io::Write;
use tar::Builder as TarBuilder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Sha256, Digest};
use chrono::Utc;

struct PackageBuilder {
    plugin_dir: PathBuf,
    output_path: PathBuf,
}

struct BuildReport {
    plugin_id: String,
    plugin_version: String,
    archive_path: PathBuf,
    archive_size_bytes: u64,
    archive_checksum: String,
    files: Vec<FileEntry>,
    built_at: chrono::DateTime<Utc>,
    validation_report: ValidationReport,
    test_report: TestReport,
}

struct FileEntry {
    path: String,
    size_bytes: u64,
    checksum: String,
}

impl PackageBuilder {
    fn build(&self) -> Result<BuildReport> {
        // 1. Validate (strict mode)
        let validation = self.validate_strict()?;
        if !validation.passed() {
            return Err(anyhow!("Validation failed with {} errors", validation.error_count()));
        }

        // 2. Run contract tests
        let tests = self.run_contract_tests()?;
        if !tests.all_passed() {
            return Err(anyhow!("Contract tests failed: {}/{} passed",
                tests.passed_count(), tests.total_count()));
        }

        // 3. Determine files to include
        let manifest = self.load_manifest()?;
        let include_patterns = vec![
            "manifest.v1.json",
            "src/**/*",
            "schemas/**/*.schema.json",
            "requirements.txt",
            "README.md",
            "LICENSE.txt",
        ];

        let mut files = Vec::new();
        for pattern in &include_patterns {
            for entry in glob::glob(&self.plugin_dir.join(pattern).to_string_lossy())? {
                let path = entry?;
                if path.is_file() {
                    let content = fs::read(&path)?;
                    let checksum = format!("sha256:{}", hex::encode(Sha256::digest(&content)));
                    files.push(FileEntry {
                        path: path.strip_prefix(&self.plugin_dir)?.to_string_lossy().to_string(),
                        size_bytes: content.len() as u64,
                        checksum,
                    });
                }
            }
        }

        // 4. Create archive (tar.gz)
        let output_file = File::create(&self.output_path)?;
        let encoder = GzEncoder::new(output_file, Compression::default());
        let mut tar = TarBuilder::new(encoder);

        for entry in &files {
            let full_path = self.plugin_dir.join(&entry.path);
            tar.append_path_with_name(&full_path, &entry.path)?;
        }

        // 5. Add checksums.txt to archive
        let checksums_content = files.iter()
            .map(|f| format!("{}  {}", f.checksum, f.path))
            .collect::<Vec<_>>()
            .join("\n");
        let mut header = tar::Header::new_gnu();
        header.set_size(checksums_content.len() as u64);
        header.set_mode(0o644);
        tar.append_data(&mut header, "checksums.txt", checksums_content.as_bytes())?;

        let encoder = tar.into_inner()?;
        let mut output_file = encoder.finish()?;
        output_file.flush()?;

        // 6. Compute archive checksum
        let archive_size = output_file.metadata()?.len();
        let archive_content = fs::read(&self.output_path)?;
        let archive_hash = format!("sha256:{}", hex::encode(Sha256::digest(&archive_content)));

        Ok(BuildReport {
            plugin_id: manifest.plugin_id,
            plugin_version: manifest.version,
            archive_path: self.output_path.clone(),
            archive_size_bytes: archive_size,
            archive_checksum: archive_hash,
            files,
            built_at: Utc::now(),
            validation_report: validation,
            test_report: tests,
        })
    }
}
```

## Tool: asset-optimizer

### CLI Specification

```
asset-optimizer 0.1.0
Optimize assets for web and Bevy with provenance tracking

USAGE:
    asset-optimizer <COMMAND>

COMMANDS:
    optimize images <dir>      Optimize images (PNG, JPG → WebP, AVIF)
    optimize fonts <dir>       Subset fonts to required characters
    optimize audio <dir>       Compress audio (WAV → OGG/MP3)
    optimize models <dir>      Optimize 3D models (glTF mesh compression)
    optimize all <dir>         Run all optimizers
    provenance <dir>           Show provenance for optimized assets
    validate <dir>             Validate optimized assets are reproducible

FLAGS:
    --source <path>            Source assets directory
    --output <path>            Output directory
    --quality <0-100>          Quality setting (default: 85)
    --provenance <path>        Write provenance manifest to path
    --dry-run                  Show what would be done without writing
    --json                     Output results as JSON
```

### Image Optimizer (Full)

```rust
// optimizer/image.rs
use std::process::Command;
use rayon::prelude::*;  // Parallel processing

struct ImageOptimizer {
    quality: u8,
    formats: Vec<OutputFormat>,
}

enum OutputFormat {
    WebP { quality: u8 },
    Avif { quality: u8 },
    PngLossless,
}

impl ImageOptimizer {
    fn optimize_directory(
        &self,
        source: &Path,
        output: &Path,
    ) -> Result<Vec<AssetRecord>> {
        let images: Vec<PathBuf> = walkdir::WalkDir::new(source)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let ext = e.path().extension().and_then(|s| s.to_str()).unwrap_or("");
                matches!(ext.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp")
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        // Process in parallel
        let records: Vec<Result<AssetRecord>> = images
            .par_iter()
            .map(|path| self.optimize_single(path, source, output))
            .collect();

        records.into_iter().collect()
    }

    fn optimize_single(
        &self,
        source_path: &Path,
        source_root: &Path,
        output_root: &Path,
    ) -> Result<AssetRecord> {
        let source_size = source_path.metadata()?.len();
        let source_hash = self.sha256_file(source_path)?;

        let rel_path = source_path.strip_prefix(source_root)?;
        let mut outputs = Vec::new();

        for format in &self.formats {
            let (ext, result) = match format {
                OutputFormat::WebP { quality } => {
                    let output_path = output_root.join(rel_path).with_extension("webp");
                    self.ensure_parent(&output_path)?;
                    self.encode_webp(source_path, &output_path, *quality)?;
                    ("webp", output_path)
                }
                OutputFormat::Avif { quality } => {
                    let output_path = output_root.join(rel_path).with_extension("avif");
                    self.ensure_parent(&output_path)?;
                    self.encode_avif(source_path, &output_path, *quality)?;
                    ("avif", output_path)
                }
                OutputFormat::PngLossless => {
                    let output_path = output_root.join(rel_path).with_extension("png");
                    self.ensure_parent(&output_path)?;
                    self.optimize_png_lossless(source_path, &output_path)?;
                    ("png", output_path)
                }
            };

            let output_size = result.metadata()?.len();
            let output_hash = self.sha256_file(&result)?;

            outputs.push(OptimizedOutput {
                path: result.strip_prefix(output_root.parent().unwrap_or(output_root))?
                    .to_string_lossy().to_string(),
                format: ext.to_string(),
                size_bytes: output_size,
                compression_ratio: if output_size > 0 {
                    (source_size as f64 / output_size as f64 * 100.0).round() / 100.0
                } else { 0.0 },
                quality: match format {
                    OutputFormat::WebP { quality } | OutputFormat::Avif { quality } => *quality,
                    OutputFormat::PngLossless => 100,
                },
                checksum: output_hash,
            });
        }

        Ok(AssetRecord {
            source: rel_path.to_string_lossy().to_string(),
            source_size_bytes: source_size,
            source_checksum: source_hash,
            outputs,
        })
    }

    fn encode_webp(&self, input: &Path, output: &Path, quality: u8) -> Result<()> {
        let status = Command::new("cwebp")
            .args(["-q", &quality.to_string()])
            .args(["-m", "6"])           // Max compression method
            .args(["-pass", "10"])       // Max quality/speed trade-off
            .args(["-mt"])                // Multi-threading
            .arg(input)
            .args(["-o", output])
            .status()?;

        if !status.success() {
            return Err(anyhow!("cwebp failed for {}", input.display()));
        }
        Ok(())
    }

    fn encode_avif(&self, input: &Path, output: &Path, quality: u8) -> Result<()> {
        let min_q = quality.saturating_sub(10);
        let status = Command::new("avifenc")
            .args(["--min", &min_q.to_string()])
            .args(["--max", &quality.to_string()])
            .args(["--speed", "6"])
            .args(["--codec", "aom"])     // AOMedia encoder
            .arg(input)
            .arg(output)
            .status()?;

        if !status.success() {
            return Err(anyhow!("avifenc failed for {}", input.display()));
        }
        Ok(())
    }

    fn optimize_png_lossless(&self, input: &Path, output: &Path) -> Result<()> {
        // Copy input to output first (oxipng modifies in place)
        fs::copy(input, output)?;

        let status = Command::new("oxipng")
            .args(["--opt", "max"])
            .args(["--strip", "all"])
            .arg(output)
            .status()?;

        if !status.success() {
            return Err(anyhow!("oxipng failed for {}", input.display()));
        }
        Ok(())
    }
}
```

### Font Optimizer (Full)

```rust
// optimizer/font.rs
struct FontOptimizer {
    // Characters required for Blup UI + learning content
    required_chars: String,
    output_formats: Vec<FontFormat>,
}

enum FontFormat {
    Woff2,
    Woff,
}

impl FontOptimizer {
    fn new() -> Self {
        // Build character set from:
        // - Latin basic + Latin-1 supplement
        // - Math operators (for KaTeX rendering)
        // - Greek letters (math/science content)
        // - Code punctuation
        // - Common UI symbols
        let mut chars = String::new();

        // ASCII printable
        chars.push_str(" !\"#$%&'()*+,-./0123456789:;<=>?@");
        chars.push_str("ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        chars.push_str("[\\]^_`");
        chars.push_str("abcdefghijklmnopqrstuvwxyz{|}~");

        // Latin-1 supplement (common in European languages)
        chars.push_str("¡¢£¤¥¦§¨©ª«¬®¯°±²³´µ¶·¸¹º»¼½¾¿");
        chars.push_str("ÀÁÂÃÄÅÆÇÈÉÊËÌÍÎÏÐÑÒÓÔÕÖ×ØÙÚÛÜÝÞß");
        chars.push_str("àáâãäåæçèéêëìíîïðñòóôõö÷øùúûüýþÿ");

        // Greek
        chars.push_str("ΑΒΓΔΕΖΗΘΙΚΛΜΝΞΟΠΡΣΤΥΦΧΨΩ");
        chars.push_str("αβγδεζηθικλμνξοπρστυφχψω");

        // Math
        chars.push_str("∀∂∃∄∅∆∇∈∉∊∋∌∍∎∏∐∑−∓∔∕∖∗∘∙√∛∜∝∞∟");
        chars.push_str("∠∡∢∣∥∦∧∨∩∪∫∬∮∯∰∱∲∴∵∶∷∸∹∺∻∼∽∾∿");
        chars.push_str("≀≁≂≃≄≅≆≇≈≉≊≋≌≍≎≏≐≑≒≓≔≕≖≗≘≙≚≛≜≝≞≟");
        chars.push_str("≠≡≢≣≤≥≦≧≨≩≪≫≬≭≮≯≰≱≲≳≴≵≶≷≸≹≺≻≼≽≾≿");

        // Arrows
        chars.push_str("←↑→↓↔↕↖↗↘↙↚↛↜↝↞↟↠↡↢↣↤↥↦↧↨↩↪↫↬↭↮↯");
        chars.push_str("↰↱↲↳↴↵↶↷↸↹↺↻↼↽↾↿⇀⇁⇂⇃⇄⇅⇆⇇⇈⇉⇊⇋⇌⇍⇎⇏");

        Self {
            required_chars: chars,
            output_formats: vec![FontFormat::Woff2],
        }
    }

    fn subset_font(&self, input: &Path, output: &Path) -> Result<AssetRecord> {
        // Use fonttools pyftsubset (Python)
        let basename = input.file_stem().unwrap().to_string_lossy();

        let woff2_path = output.join(format!("{}.woff2", basename));
        self.ensure_parent(&woff2_path)?;

        let status = Command::new("pyftsubset")
            .arg(input)
            .args(["--text", &self.required_chars])
            .args(["--output-file", &woff2_path.to_string_lossy().to_string()])
            .args(["--flavor", "woff2"])
            .args(["--layout-features", "*"])
            .args(["--name-IDs", "*"])  // Keep all name records
            .arg("--desubroutinize")
            .arg("--no-hinting")
            .status()?;

        if !status.success() {
            return Err(anyhow!("pyftsubset failed for {}", input.display()));
        }

        let source_size = input.metadata()?.len();
        let output_size = woff2_path.metadata()?.len();

        Ok(AssetRecord {
            source: input.file_name().unwrap().to_string_lossy().to_string(),
            source_size_bytes: source_size,
            source_checksum: self.sha256_file(input)?,
            outputs: vec![OptimizedOutput {
                path: woff2_path.file_name().unwrap().to_string_lossy().to_string(),
                format: "woff2".into(),
                size_bytes: output_size,
                compression_ratio: (source_size as f64 / output_size as f64 * 100.0).round() / 100.0,
                quality: 100, // Lossless
                checksum: self.sha256_file(&woff2_path)?,
            }],
        })
    }
}
```

### Provenance System

The provenance manifest enables reproducibility and audit:

```rust
// provenance.rs
#[derive(Serialize, Deserialize)]
struct ProvenanceManifest {
    optimizer_version: String,
    optimized_at: chrono::DateTime<chrono::Utc>,
    git_commit: Option<String>,
    host: String,
    total_source_bytes: u64,
    total_optimized_bytes: u64,
    overall_compression_ratio: f64,
    records: Vec<AssetRecord>,
}

impl ProvenanceManifest {
    fn write(&self, output_dir: &Path) -> Result<()> {
        let path = output_dir.join("provenance.json");

        // Read existing manifest if present (for incremental updates)
        let mut existing = if path.exists() {
            serde_json::from_reader::<_, Vec<AssetRecord>>(File::open(&path)?)?
        } else {
            Vec::new()
        };

        // Merge: update existing entries, add new ones
        for record in &self.records {
            if let Some(existing_record) = existing.iter_mut()
                .find(|r| r.source == record.source)
            {
                *existing_record = record.clone();
            } else {
                existing.push(record.clone());
            }
        }

        let json = serde_json::to_string_pretty(&self)?;
        fs::write(&path, json)?;

        tracing::info!(
            path = %path.display(),
            records = existing.len(),
            "Provenance manifest written"
        );
        Ok(())
    }

    /// Verify that re-optimizing produces identical output (reproducibility check)
    fn verify_reproducibility(&self, output_dir: &Path) -> Result<ReproducibilityReport> {
        let mut report = ReproducibilityReport::default();

        for record in &self.records {
            for output in &record.outputs {
                let path = output_dir.join(&output.path);
                if !path.exists() {
                    report.missing.push(output.path.clone());
                    continue;
                }

                let current_hash = sha256_file(&path)?;
                if current_hash == output.checksum {
                    report.verified.push(output.path.clone());
                } else {
                    report.mismatched.push(ReproducibilityMismatch {
                        path: output.path.clone(),
                        expected: output.checksum.clone(),
                        actual: current_hash,
                    });
                }
            }
        }

        Ok(report)
    }
}
```

## Testing Strategy

### plugin-builder Tests

| Test | Method | Expected |
|------|--------|----------|
| Valid plugin passes validation | Reference plugin fixture | Exit 0, no errors |
| Missing manifest | Plugin dir without manifest | "manifest.v1.json not found" |
| Invalid manifest JSON | Malformed JSON in manifest | "Invalid JSON at line X" |
| Unknown permission | Permission not in allowed set | "Unknown permission: X" |
| Missing entry point | Manifest references nonexistent file | "Entry point not found: X" |
| Build produces valid archive | `build` command on valid plugin | tar.gz with correct structure |
| Archive contains checksums.txt | Inspect built archive | Every file has SHA-256 entry |
| Contract test catches malformed request | Mock plugin that validates inputs | Test: "malformed_request" → fails |
| Contract test catches permission denial | Mock plugin, request without perms | Test: "permission/deny_X" → passes |
| Build fails if contract tests fail | Plugin with failing tests | Build aborted, "Contract tests failed" message |

### asset-optimizer Tests

| Test | Method | Expected |
|------|--------|----------|
| PNG → WebP size reduction | Known 200KB PNG | WebP < 100KB |
| AVIF compresses more than WebP | Same image | AVIF size < WebP size |
| PNG lossless is lossless | Pixel-compare with source | Identical pixels |
| Font subsetting reduces size | 400KB font → subset | Output < 50KB |
| Subset font contains required chars | Render test string | All chars render |
| Provenance manifest is valid JSON | `provenance` command | Valid JSON, all records present |
| Reproducibility check | Same input → same checksum | `validate` command passes |
| Dry run writes nothing | `--dry-run` flag | Output dir unchanged |
| Large file (10MB) handled | 10MB PNG | No OOM, successful output |
| Invalid font file → error | Corrupt .ttf file | Clear error, non-zero exit |

## Quality Gates

- [ ] `plugin-builder validate` passes on a reference plugin with zero errors
- [ ] `plugin-builder build` produces a valid tar.gz with all required files
- [ ] Contract tests cover every declared capability + every forbidden permission
- [ ] `asset-optimizer optimize images` produces valid WebP/AVIF for all supported formats
- [ ] `asset-optimizer optimize fonts` produces subset fonts that contain all required chars
- [ ] All optimizations are reproducible (verified by `asset-optimizer validate`)
- [ ] Provenance manifest is complete, valid JSON, and committed alongside assets
- [ ] No source assets are overwritten (output goes to separate directory)
- [ ] All tools exit with non-zero code and clear error message on failure
