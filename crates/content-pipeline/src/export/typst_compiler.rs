use std::collections::HashMap;

use sandbox_manager::models::limits::SandboxLimits;
use sandbox_manager::models::request::ToolKind;
use sandbox_manager::{ExecutionStatus, SandboxManager, SandboxRequest};

use crate::error::{DiagnosticSeverity, ExportError, TypstDiagnostic};
use crate::models::document_artifact::DocumentArtifact;

pub struct TypstCompiler {
    sandbox: SandboxManager,
}

impl TypstCompiler {
    pub fn new(sandbox: SandboxManager) -> Self {
        Self { sandbox }
    }

    /// Compile Typst source to PDF via sandbox
    pub async fn compile_to_pdf(
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

        let command = format!(
            "{}echo '{}' | base64 -d > /workspace/input.typst && \
             typst compile /workspace/input.typst /workspace/output.pdf 2>&1 && \
             cat /workspace/output.pdf | base64",
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

                // Count pages
                let page_count = count_pdf_pages(&pdf_data);

                let mut artifact = DocumentArtifact::new_pdf(&pdf_data, typst_source);
                artifact.page_count = Some(page_count);

                Ok(artifact)
            }
            ExecutionStatus::TimeoutCompile => Err(ExportError::CompilationFailed {
                message: "Compilation timed out".to_string(),
                diagnostics: Vec::new(),
            }),
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

fn count_pdf_pages(data: &[u8]) -> u32 {
    let text = String::from_utf8_lossy(data);
    // Count page objects: look for "/Type /Page" not followed by "s"
    let re = regex::Regex::new(r"/Type\s*/Page[^s]").unwrap();
    re.find_iter(&text).count() as u32
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

            let message = line.splitn(2, ':').nth(1).unwrap_or("").trim().to_string();

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
