use thiserror::Error;

#[derive(Error, Debug)]
pub enum BuildError {
    #[error("Docker build failed: {0}")]
    DockerBuildFailed(String),

    #[error("Definition not found: {0}")]
    DefinitionNotFound(String),

    #[error("Invalid definition: {0}")]
    InvalidDefinition(String),

    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    #[error("Scan failed: {0}")]
    ScanFailed(String),

    #[error("Test failed: {0}")]
    TestFailed(String),

    #[error("Clean failed: {0}")]
    CleanFailed(String),

    #[error("IO error: {0}")]
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            BuildError::DockerBuildFailed("oops".to_string()).to_string(),
            "Docker build failed: oops"
        );
        assert_eq!(
            BuildError::DefinitionNotFound("python.yaml".to_string()).to_string(),
            "Definition not found: python.yaml"
        );
    }

    #[test]
    fn test_error_debug() {
        let err = BuildError::TestFailed("assertion failed".to_string());
        let debug = format!("{err:?}");
        assert!(debug.contains("TestFailed"));
        assert!(debug.contains("assertion failed"));
    }

    #[test]
    fn test_all_error_variants() {
        let variants = [
            BuildError::DockerBuildFailed("d".into()),
            BuildError::DefinitionNotFound("d".into()),
            BuildError::InvalidDefinition("d".into()),
            BuildError::VerificationFailed("d".into()),
            BuildError::ScanFailed("d".into()),
            BuildError::TestFailed("d".into()),
            BuildError::CleanFailed("d".into()),
            BuildError::IoError("d".into()),
        ];
        assert_eq!(variants.len(), 8);
        for v in &variants {
            assert!(!v.to_string().is_empty());
        }
    }
}
