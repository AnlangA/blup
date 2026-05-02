use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BuildConfig {
    pub definitions_dir: PathBuf,
    pub dockerfiles_dir: PathBuf,
    pub tag: String,
    pub no_cache: bool,
    pub push: bool,
    pub registry: Option<String>,
    pub platform: String,
    pub output: Option<PathBuf>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            definitions_dir: PathBuf::from("sandboxes/definitions"),
            dockerfiles_dir: PathBuf::from("sandboxes/docker"),
            tag: "latest".to_string(),
            no_cache: false,
            push: false,
            registry: None,
            platform: "linux/amd64".to_string(),
            output: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        let config = BuildConfig::default();
        assert_eq!(
            config.definitions_dir,
            PathBuf::from("sandboxes/definitions")
        );
        assert_eq!(config.dockerfiles_dir, PathBuf::from("sandboxes/docker"));
        assert_eq!(config.tag, "latest");
        assert!(!config.no_cache);
        assert!(!config.push);
        assert!(config.registry.is_none());
        assert_eq!(config.platform, "linux/amd64");
        assert!(config.output.is_none());
    }

    #[test]
    fn test_custom_config() {
        let config = BuildConfig {
            tag: "v1.0".to_string(),
            no_cache: true,
            platform: "linux/arm64".to_string(),
            ..Default::default()
        };
        assert_eq!(config.tag, "v1.0");
        assert!(config.no_cache);
        assert_eq!(config.platform, "linux/arm64");
    }

    #[test]
    fn test_registry_config() {
        let config = BuildConfig {
            registry: Some("ghcr.io".to_string()),
            push: true,
            ..Default::default()
        };
        assert_eq!(config.registry.unwrap(), "ghcr.io");
        assert!(config.push);
    }
}
