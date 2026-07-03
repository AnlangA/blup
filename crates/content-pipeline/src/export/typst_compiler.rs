use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use sandbox_manager::models::limits::SandboxLimits;
use sandbox_manager::{ExecutionStatus, SandboxManager, SandboxRequest, ToolKind};

use crate::error::{DiagnosticSeverity, ExportError, TypstDiagnostic};
use crate::models::document_artifact::DocumentArtifact;

pub struct TypstCompiler {
    sandbox: Arc<SandboxManager>,
    allow_host_cli_fallback: bool,
}

impl TypstCompiler {
    pub fn new(sandbox: Arc<SandboxManager>) -> Self {
        Self {
            sandbox,
            allow_host_cli_fallback: false,
        }
    }

    /// Allow falling back to the host `typst` CLI when sandbox compilation fails.
    ///
    /// This is intended for explicit developer tooling only. Product export paths
    /// should keep the default sandbox-only behavior so private lesson content is
    /// not compiled on the host by surprise.
    pub fn with_host_cli_fallback(mut self, allow: bool) -> Self {
        self.allow_host_cli_fallback = allow;
        self
    }

    /// Compile Typst source to PDF via sandbox.
    ///
    /// Host CLI fallback is disabled by default and must be enabled explicitly
    /// with [`TypstCompiler::with_host_cli_fallback`].
    pub async fn compile_to_pdf(
        &self,
        typst_source: &str,
        assets: &HashMap<String, Vec<u8>>,
    ) -> Result<DocumentArtifact, ExportError> {
        match self.compile_via_sandbox(typst_source, assets).await {
            Ok(artifact) => return Ok(artifact),
            Err(sandbox_err) => {
                if !self.allow_host_cli_fallback {
                    return Err(sandbox_err);
                }
                tracing::debug!(
                    error = %sandbox_err,
                    "Sandbox compilation unavailable, trying host typst CLI"
                );
            }
        }

        // Fall back to host typst CLI
        compile_via_cli(typst_source)
    }

    /// Compile Typst source to PDF via Docker sandbox.
    async fn compile_via_sandbox(
        &self,
        typst_source: &str,
        assets: &HashMap<String, Vec<u8>>,
    ) -> Result<DocumentArtifact, ExportError> {
        // Build sandbox command: write assets, compile, output PDF
        let mut setup_commands = String::new();

        for (name, data) in assets {
            let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data);
            setup_commands.push_str(&format!(
                "echo '{}' | base64 -d > /workspace/{} && ",
                encoded, name
            ));
        }

        let encoded_source = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            typst_source.as_bytes(),
        );

        // Redirect typst stderr to a file instead of stdout — otherwise
        // warnings (e.g. font fallback) mix with the base64 PDF and corrupt it.
        // On failure the error file is piped to stderr so parse_typst_errors can read it.
        let command = format!(
            "{}echo '{}' | base64 -d > /workspace/input.typst && \
             typst compile /workspace/input.typst /workspace/output.pdf >/dev/null 2>/workspace/errors.txt; \
             EXIT=$?; \
             if [ $EXIT -eq 0 ]; then \
               base64 /workspace/output.pdf; \
             else \
               cat /workspace/errors.txt >&2; \
               exit $EXIT; \
             fi",
            setup_commands, encoded_source,
        );

        let request = SandboxRequest {
            request_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            tool_kind: ToolKind::TypstCompile,
            code: command,
            language: Some("typst".to_string()),
            limits: SandboxLimits {
                compile_timeout_secs: 60,
                memory_mb: 1024,
                ..SandboxLimits::default()
            },
            stdin: None,
            environment: None,
        };

        let result = self
            .sandbox
            .execute(request)
            .await
            .map_err(|e| ExportError::Sandbox(e.to_string()))?;

        match result.status {
            ExecutionStatus::Success => {
                // Decode base64 PDF from stdout
                let pdf_data = base64::Engine::decode(
                    &base64::engine::general_purpose::STANDARD,
                    result.stdout.trim(),
                )
                .map_err(|e| ExportError::InvalidPdfOutput(e.to_string()))?;

                // Validate PDF header
                if pdf_data.len() < 5 || &pdf_data[..5] != b"%PDF-" {
                    return Err(ExportError::InvalidPdfOutput(
                        "Output does not start with %PDF-".to_string(),
                    ));
                }

                let mut artifact = DocumentArtifact::new_pdf(&pdf_data, typst_source);
                artifact.page_count = count_pdf_pages(&pdf_data);

                Ok(artifact)
            }
            ExecutionStatus::TimeoutCompile | ExecutionStatus::TimeoutRun => {
                Err(ExportError::CompilationFailed {
                    message: "Compilation timed out".to_string(),
                    diagnostics: Vec::new(),
                })
            }
            _ => {
                let diagnostics = parse_typst_errors(&result.stderr);
                Err(ExportError::CompilationFailed {
                    message: "Typst compilation failed".to_string(),
                    diagnostics,
                })
            }
        }
    }
}

/// Compile Typst source to PDF using the host `typst` CLI.
fn compile_via_cli(typst_source: &str) -> Result<DocumentArtifact, ExportError> {
    let tmp_dir = tempfile::TempDir::new().map_err(|e| {
        ExportError::Io(std::io::Error::other(format!(
            "Failed to create temp dir: {e}"
        )))
    })?;

    let input_path = tmp_dir.path().join("input.typ");
    let output_path = tmp_dir.path().join("output.pdf");

    std::fs::write(&input_path, typst_source).map_err(ExportError::Io)?;

    let output = std::process::Command::new("typst")
        .args([
            "compile",
            input_path.to_str().unwrap_or("input.typ"),
            output_path.to_str().unwrap_or("output.pdf"),
        ])
        .output()
        .map_err(|e| ExportError::CompilationFailed {
            message: format!("Failed to run typst CLI: {e}"),
            diagnostics: Vec::new(),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let diagnostics = parse_typst_errors(&stderr);
        return Err(ExportError::CompilationFailed {
            message: "Host typst CLI compilation failed".to_string(),
            diagnostics,
        });
    }

    let pdf_data = std::fs::read(&output_path).map_err(ExportError::Io)?;

    if pdf_data.len() < 5 || &pdf_data[..5] != b"%PDF-" {
        return Err(ExportError::InvalidPdfOutput(
            "CLI output does not start with %PDF-".to_string(),
        ));
    }

    let mut artifact = DocumentArtifact::new_pdf(&pdf_data, typst_source);
    artifact.page_count = count_pdf_pages(&pdf_data);
    Ok(artifact)
}

fn count_pdf_pages(data: &[u8]) -> Option<u32> {
    let text = String::from_utf8_lossy(data);
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE
        .get_or_init(|| regex::Regex::new(r"/Type\s*/Page(\s|/|>>|\])").expect("valid page regex"));
    let count = re.find_iter(&text).count() as u32;
    (count > 0).then_some(count)
}

fn parse_typst_errors(stderr: &str) -> Vec<TypstDiagnostic> {
    let mut diagnostics = Vec::new();

    let lines: Vec<&str> = stderr.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if line.starts_with("error:") || line.starts_with("warning:") {
            let severity = if line.starts_with("error:") {
                DiagnosticSeverity::Error
            } else {
                DiagnosticSeverity::Warning
            };

            let message = line
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("")
                .trim()
                .to_string();

            let mut diagnostic = TypstDiagnostic {
                severity,
                message,
                line: None,
                column: None,
                source_line: None,
                hint: None,
            };

            // Try to parse location from next line
            if i + 1 < lines.len() {
                let location_line = lines[i + 1];
                if location_line.contains("input.typ") {
                    let parts: Vec<&str> = location_line.split(':').collect();
                    if parts.len() >= 3 {
                        diagnostic.line = parts[1].trim().parse().ok();
                        diagnostic.column = parts[2].trim().parse().ok();
                    }
                }
            }

            diagnostics.push(diagnostic);
        }

        i += 1;
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use sandbox_manager::models::result::SandboxResult;
    use sandbox_manager::models::status::ExecutionStatus;
    use sandbox_manager::{MockExecutor, SandboxManager};

    #[tokio::test]
    async fn sandbox_failure_does_not_use_host_cli_by_default() {
        let mut mock = MockExecutor::new();
        mock.set_response_fn(Box::new(|req| SandboxResult {
            request_id: req.request_id,
            session_id: Some(req.session_id),
            status: ExecutionStatus::InternalError,
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "sandbox compile failed".to_string(),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms: 1,
            resource_usage: Default::default(),
            error: None,
        }));

        let sandbox = Arc::new(SandboxManager::with_executor(Box::new(mock)));
        let compiler = TypstCompiler::new(sandbox);
        let err = compiler
            .compile_to_pdf("#set page(width: 10cm)\n= Test", &HashMap::new())
            .await
            .expect_err("sandbox failure should be returned directly");

        match err {
            ExportError::CompilationFailed { message, .. } => {
                assert_eq!(message, "Typst compilation failed");
            }
            other => panic!("expected compilation failure, got {other:?}"),
        }
    }
}
