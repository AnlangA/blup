use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    #[error("Validation failed for '{schema}': {errors}")]
    ValidationFailed { schema: String, errors: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Schema compile error for '{schema}': {message}")]
    SchemaCompile { schema: String, message: String },
}

pub struct SchemaValidator {
    schemas_dir: PathBuf,
    cache: RwLock<HashMap<String, Arc<jsonschema::Validator>>>,
}

impl SchemaValidator {
    pub fn new<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            schemas_dir: dir.as_ref().to_path_buf(),
            cache: RwLock::new(HashMap::new()),
        }
    }

    fn load_and_compile(
        &self,
        schema_name: &str,
    ) -> Result<Arc<jsonschema::Validator>, ValidationError> {
        {
            let cache = self.cache.read().unwrap();
            if let Some(compiled) = cache.get(schema_name) {
                return Ok(Arc::clone(compiled));
            }
        }

        let filename = format!("{}.v1.schema.json", schema_name);
        let path = self.schemas_dir.join(&filename);

        if !path.exists() {
            return Err(ValidationError::SchemaNotFound(schema_name.to_string()));
        }

        let schema_content = std::fs::read_to_string(&path)?;
        let schema_json: serde_json::Value = serde_json::from_str(&schema_content)?;

        let compiled = jsonschema::Validator::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&schema_json)
            .map_err(|e| ValidationError::SchemaCompile {
                schema: schema_name.to_string(),
                message: e.to_string(),
            })?;

        let compiled = Arc::new(compiled);
        let mut cache = self.cache.write().unwrap();
        cache.insert(schema_name.to_string(), Arc::clone(&compiled));

        Ok(compiled)
    }

    /// Validate a JSON value against a named schema.
    /// Returns `Ok(())` if valid, `Err(ValidationError)` if not.
    pub fn validate(
        &self,
        data: &serde_json::Value,
        schema_name: &str,
    ) -> Result<(), ValidationError> {
        let compiled = self.load_and_compile(schema_name)?;
        let errors: Vec<String> = compiled.iter_errors(data).map(|e| e.to_string()).collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(ValidationError::ValidationFailed {
                schema: schema_name.to_string(),
                errors: errors.join("; "),
            })
        }
    }

    /// Validate and return a clone of the data if valid.
    pub fn validate_owned(
        &self,
        data: serde_json::Value,
        schema_name: &str,
    ) -> Result<serde_json::Value, ValidationError> {
        self.validate(&data, schema_name)?;
        Ok(data)
    }

    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
    }
}
