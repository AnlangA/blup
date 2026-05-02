use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::process::Command;

use crate::config::BuildConfig;
use crate::error::BuildError;

#[derive(Debug, Serialize, Deserialize)]
pub struct SandboxDefinition {
    pub name: String,
    pub description: String,
    pub dockerfile: String,
    pub version: String,
    pub base_image: BaseImage,
    pub pinned_packages: Vec<String>,
    pub build_args: BuildArgs,
    pub security: SecurityConfig,
    pub limits: LimitsConfig,
    pub tests: Vec<ImageTest>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BaseImage {
    pub name: String,
    pub digest: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildArgs {
    pub user_id: String,
    pub user_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub seccomp_profile: String,
    pub read_only_rootfs: bool,
    pub no_new_privileges: bool,
    pub cap_drop: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LimitsConfig {
    pub compile_timeout_secs: u32,
    pub run_timeout_secs: u32,
    pub memory_mb: u32,
    pub cpu_count: f64,
    pub disk_mb: u32,
    pub max_processes: u32,
    pub network_enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageTest {
    pub name: String,
    pub command: Vec<String>,
    pub expected_output: Option<String>,
    pub expected_exit_code: Option<i32>,
}

pub struct SandboxBuilder {
    config: BuildConfig,
}

impl SandboxBuilder {
    pub fn new(config: BuildConfig) -> Self {
        Self { config }
    }

    pub async fn build(&self, sandbox_name: &str) -> Result<(), BuildError> {
        let definition = self.load_definition(sandbox_name)?;
        let image_tag = self.compute_image_tag(&definition);

        tracing::info!(sandbox = sandbox_name, tag = %image_tag, "Building sandbox image");

        // Build the image
        let dockerfile_path = self.config.dockerfiles_dir.join(&definition.dockerfile);
        let status = Command::new("docker")
            .args([
                "build",
                "--file",
                &dockerfile_path.to_string_lossy(),
                "--tag",
                &image_tag,
                "--platform",
                &self.config.platform,
                "--build-arg",
                &format!("USER_ID={}", definition.build_args.user_id),
                "--build-arg",
                &format!("USER_NAME={}", definition.build_args.user_name),
            ])
            .arg(if self.config.no_cache {
                "--no-cache"
            } else {
                ""
            })
            .arg(".")
            .status()
            .map_err(|e| {
                BuildError::DockerBuildFailed(format!("Failed to run docker build: {}", e))
            })?;

        if !status.success() {
            return Err(BuildError::DockerBuildFailed(
                "Docker build failed".to_string(),
            ));
        }

        // Run verification tests
        let test_results = self
            .run_verification_tests(&image_tag, &definition.tests)
            .await?;
        let all_passed = test_results.iter().all(|r| r.passed);

        if !all_passed {
            return Err(BuildError::VerificationFailed(
                "Some verification tests failed".to_string(),
            ));
        }

        tracing::info!(sandbox = sandbox_name, tag = %image_tag, "Sandbox image built successfully");

        Ok(())
    }

    pub async fn build_all(&self) -> Result<(), BuildError> {
        let definitions = self.list_definitions()?;
        for definition in definitions {
            self.build(&definition).await?;
        }
        Ok(())
    }

    pub async fn scan(&self, image: &str) -> Result<(), BuildError> {
        tracing::info!(image = image, "Scanning image for vulnerabilities");

        let output = Command::new("trivy")
            .args([
                "image",
                "--format",
                "json",
                "--severity",
                "HIGH,CRITICAL",
                "--ignore-unfixed",
                image,
            ])
            .output()
            .map_err(|e| BuildError::ScanFailed(format!("Failed to run trivy: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::ScanFailed(format!(
                "Trivy scan failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);

        Ok(())
    }

    pub async fn verify(&self, image: &str) -> Result<(), BuildError> {
        tracing::info!(image = image, "Verifying image digest");

        let output = Command::new("docker")
            .args(["inspect", "--format={{index .RepoDigests 0}}", image])
            .output()
            .map_err(|e| {
                BuildError::VerificationFailed(format!("Failed to inspect image: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::VerificationFailed(format!(
                "Failed to inspect image: {}",
                stderr
            )));
        }

        let digest = String::from_utf8_lossy(&output.stdout);
        println!("Image digest: {}", digest);

        Ok(())
    }

    pub async fn list(&self) -> Result<(), BuildError> {
        let definitions = self.list_definitions()?;
        println!("Available sandbox definitions:");
        for name in definitions {
            println!("  - {}", name);
        }
        Ok(())
    }

    pub async fn clean(&self) -> Result<(), BuildError> {
        tracing::info!("Cleaning built sandbox images");

        let output = Command::new("docker")
            .args(["images", "--filter", "reference=blup/sandbox-*", "-q"])
            .output()
            .map_err(|e| BuildError::CleanFailed(format!("Failed to list images: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let image_ids: Vec<&str> = stdout.lines().filter(|s| !s.is_empty()).collect();

        for image_id in &image_ids {
            let status = Command::new("docker")
                .args(["rmi", "-f", image_id])
                .status()
                .map_err(|e| BuildError::CleanFailed(format!("Failed to remove image: {}", e)))?;

            if !status.success() {
                tracing::warn!(image_id = image_id, "Failed to remove image");
            }
        }

        tracing::info!("Cleaned {} images", image_ids.len());
        Ok(())
    }

    fn load_definition(&self, name: &str) -> Result<SandboxDefinition, BuildError> {
        let definition_path = self.config.definitions_dir.join(format!("{}.yaml", name));
        let content = std::fs::read_to_string(&definition_path)
            .map_err(|e| BuildError::DefinitionNotFound(format!("{}: {}", name, e)))?;

        let definition: SandboxDefinition = serde_yaml::from_str(&content)
            .map_err(|e| BuildError::InvalidDefinition(format!("{}: {}", name, e)))?;

        Ok(definition)
    }

    fn list_definitions(&self) -> Result<Vec<String>, BuildError> {
        let mut definitions = Vec::new();

        for entry in std::fs::read_dir(&self.config.definitions_dir)
            .map_err(|e| BuildError::IoError(format!("Failed to read definitions dir: {}", e)))?
        {
            let entry =
                entry.map_err(|e| BuildError::IoError(format!("Failed to read entry: {}", e)))?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "yaml") {
                if let Some(name) = path.file_stem() {
                    definitions.push(name.to_string_lossy().to_string());
                }
            }
        }

        Ok(definitions)
    }

    fn compute_image_tag(&self, definition: &SandboxDefinition) -> String {
        let content = serde_json::to_string(definition).unwrap_or_default();
        let hash = Sha256::digest(content.as_bytes());
        let short_hash = hex::encode(&hash[..8]);

        if let Some(ref registry) = self.config.registry {
            format!(
                "{}/blup/{}:v{}-sha256:{}",
                registry, definition.name, definition.version, short_hash
            )
        } else {
            format!(
                "blup/{}:v{}-sha256:{}",
                definition.name, definition.version, short_hash
            )
        }
    }

    async fn run_verification_tests(
        &self,
        image_tag: &str,
        tests: &[ImageTest],
    ) -> Result<Vec<TestResult>, BuildError> {
        let mut results = Vec::new();

        for test in tests {
            let mut cmd = Command::new("docker");
            cmd.args(["run", "--rm", image_tag]);
            cmd.args(&test.command);

            let output = cmd
                .output()
                .map_err(|e| BuildError::TestFailed(format!("{}: {}", test.name, e)))?;

            let passed = match (&test.expected_output, &test.expected_exit_code) {
                (Some(expected), _) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    stdout.trim().contains(expected.as_str())
                }
                (_, Some(expected_code)) => output.status.code() == Some(*expected_code),
                _ => output.status.success(),
            };

            results.push(TestResult {
                name: test.name.clone(),
                passed,
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_code: output.status.code(),
            });

            if !passed {
                tracing::warn!(test = %test.name, "Image verification test failed");
            }
        }

        Ok(results)
    }
}

#[allow(dead_code)]
struct TestResult {
    name: String,
    passed: bool,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}
