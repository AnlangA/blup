use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
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
/// Templates are cached in memory after first load.
pub struct PromptLoader {
    templates_dir: PathBuf,
    shared_partials: Vec<String>,
    cache: RwLock<HashMap<String, String>>,
}

impl PromptLoader {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        let templates_dir = dir.as_ref().to_path_buf();
        let shared_partials = Self::load_shared_partials(&templates_dir);
        Self {
            templates_dir,
            shared_partials,
            cache: RwLock::new(HashMap::new()),
        }
    }

    fn load_shared_partials(dir: &Path) -> Vec<String> {
        let shared_dir = dir.join("shared");
        let mut partials = Vec::new();

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

    /// Load a prompt template by name and version, with caching.
    fn load_inner(&self, name: &str, version: u32) -> Result<String, PromptError> {
        let cache_key = format!("{}.v{}", name, version);

        {
            let cache = self.cache.read().expect("RwLock poisoned");
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let filename = format!("{}.v{}.prompt.md", name, version);
        let path = self.templates_dir.join(&filename);

        if !path.exists() {
            return Err(PromptError::NotFound {
                name: name.to_string(),
                version,
            });
        }

        let template = std::fs::read_to_string(path)?;
        let trimmed = template.trim().to_string();

        let mut cache = self.cache.write().expect("RwLock poisoned");
        cache.insert(cache_key, trimmed.clone());

        Ok(trimmed)
    }

    /// Load a prompt template by name and version.
    pub fn load(&self, name: &str, version: u32) -> Result<String, PromptError> {
        self.load_inner(name, version)
    }

    /// Load a prompt and render it with variable substitution.
    ///
    /// Combines shared partials + the prompt template, then substitutes
    /// `{{variable_name}}` placeholders with provided values.
    pub fn load_and_render(
        &self,
        name: &str,
        version: u32,
        vars: &HashMap<String, String>,
    ) -> Result<String, PromptError> {
        let template = self.load_inner(name, version)?;
        let mut parts: Vec<&str> = Vec::new();

        for partial in &self.shared_partials {
            parts.push(partial.as_str());
        }

        parts.push(&template);

        let combined = parts.join("\n\n---\n\n");
        let rendered = Self::render_template(&combined, vars);

        Ok(rendered)
    }

    /// Render variables into a template string.
    pub fn render(&self, template: &str, vars: &HashMap<String, String>) -> String {
        Self::render_template(template, vars)
    }

    fn render_template(template: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in vars {
            let escaped = html_escape(value);
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, &escaped);
        }
        result
    }

    /// Reload shared partials and clear cache (useful during development).
    pub fn reload_partials(&mut self) {
        self.shared_partials = Self::load_shared_partials(&self.templates_dir);
        self.cache.write().expect("RwLock poisoned").clear();
    }
}

/// Minimal HTML-escaping for user-provided values inserted into prompts.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
