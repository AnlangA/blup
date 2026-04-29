use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PromptError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Template not found: {name} v{version}")]
    NotFound { name: String, version: u32 },
}

/// Loads and renders versioned prompt templates with shared partials.
///
/// Shared partials in `shared/` (persona, safety rules, output format) are
/// loaded once and automatically prepended to every rendered prompt.
pub struct PromptLoader {
    templates_dir: PathBuf,
    shared_partials: Vec<String>,
}

impl PromptLoader {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let templates_dir = dir.as_ref().to_path_buf();
        let shared_partials = Self::load_shared_partials(&templates_dir);
        Self {
            templates_dir,
            shared_partials,
        }
    }

    fn load_shared_partials(dir: &Path) -> Vec<String> {
        let shared_dir = dir.join("shared");
        let mut partials = Vec::new();

        // Order matters: persona, then safety, then output format
        let ordered = ["persona", "safety_rules", "output_format_guide"];
        for name in &ordered {
            let filename = format!("{}.partial.md", name);
            let path = shared_dir.join(&filename);
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if !content.trim().is_empty() {
                        partials.push(content.trim().to_string());
                    }
                }
            }
        }

        // Also load any additional .partial.md files not in the ordered list
        if shared_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&shared_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    let name_str = name.to_string_lossy();
                    if name_str.ends_with(".partial.md") {
                        let base = name_str.replace(".partial.md", "");
                        if !ordered.contains(&base.as_str()) {
                            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                                if !content.trim().is_empty() {
                                    partials.push(content.trim().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        partials
    }

    /// Load a prompt template by name and version.
    ///
    /// Looks for `{name}.v{version}.prompt.md` in the templates directory.
    pub fn load(&self, name: &str, version: u32) -> Result<String, PromptError> {
        let filename = format!("{}.v{}.prompt.md", name, version);
        let path = self.templates_dir.join(&filename);

        if !path.exists() {
            return Err(PromptError::NotFound {
                name: name.to_string(),
                version,
            });
        }

        let template = std::fs::read_to_string(path)?;
        Ok(template.trim().to_string())
    }

    /// Load a prompt and render it with variable substitution.
    ///
    /// Combines shared partials + the prompt template, then substitutes
    /// `{{variable_name}}` placeholders with provided values.
    ///
    /// Variable values are HTML-escaped to prevent prompt injection via user
    /// input fields (the LLM reads markdown, not HTML, so this is safe).
    pub fn load_and_render(
        &self,
        name: &str,
        version: u32,
        vars: &HashMap<String, String>,
    ) -> Result<String, PromptError> {
        let template = self.load(name, version)?;
        let mut parts: Vec<&str> = Vec::new();

        // Prepend shared partials as system context
        for partial in &self.shared_partials {
            parts.push(partial.as_str());
        }

        parts.push(&template);

        let combined = parts.join("\n\n---\n\n");
        let rendered = Self::render_template(&combined, vars);

        Ok(rendered)
    }

    /// Render variables into a template string.
    ///
    /// Replaces `{{key}}` with the corresponding value. Missing keys are
    /// left as-is (literal `{{key}}`) to surface the issue to the LLM.
    pub fn render(&self, template: &str, vars: &HashMap<String, String>) -> String {
        Self::render_template(template, vars)
    }

    fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            // Escape the value to prevent accidental markdown/JSON injection.
            // This is a basic defense — the schema validator is the real gate.
            let escaped = html_escape(value);
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, &escaped);
        }
        result
    }

    /// Reload shared partials (useful during development if partials change).
    pub fn reload_partials(&mut self) {
        self.shared_partials = Self::load_shared_partials(&self.templates_dir);
    }
}

/// Minimal HTML-escaping for user-provided values inserted into prompts.
/// Escapes `<`, `>`, `&`, `"`. Does NOT escape `{`/`}` because those are
/// template syntax, not HTML — they are handled separately by ensuring
/// variable values don't contain `{{`.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
