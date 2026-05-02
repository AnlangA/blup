use crate::error::SandboxError;
use std::process::Command;

pub struct DockerClient {
    docker_cmd: String,
}

impl DockerClient {
    pub fn new() -> Self {
        Self {
            docker_cmd: "docker".to_string(),
        }
    }

    pub fn with_command(mut self, cmd: &str) -> Self {
        self.docker_cmd = cmd.to_string();
        self
    }

    pub fn health_check(&self) -> Result<(), SandboxError> {
        let output = Command::new(&self.docker_cmd)
            .args(["info"])
            .output()
            .map_err(|e| SandboxError::docker(&format!("Failed to run docker info: {}", e)))?;

        if !output.status.success() {
            return Err(SandboxError::docker("Docker daemon is not running"));
        }

        Ok(())
    }

    pub fn pull_image(&self, image: &str) -> Result<(), SandboxError> {
        let output = Command::new(&self.docker_cmd)
            .args(["pull", image])
            .output()
            .map_err(|e| SandboxError::docker(&format!("Failed to pull image: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SandboxError::docker(&format!(
                "Failed to pull image {}: {}",
                image, stderr
            )));
        }

        Ok(())
    }

    pub fn remove_container(&self, name: &str) -> Result<(), SandboxError> {
        let output = Command::new(&self.docker_cmd)
            .args(["rm", "-f", name])
            .output()
            .map_err(|e| SandboxError::docker(&format!("Failed to remove container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SandboxError::docker(&format!(
                "Failed to remove container {}: {}",
                name, stderr
            )));
        }

        Ok(())
    }

    pub fn kill_container(&self, name: &str) -> Result<(), SandboxError> {
        let output = Command::new(&self.docker_cmd)
            .args(["kill", "--signal=SIGKILL", name])
            .output()
            .map_err(|e| SandboxError::docker(&format!("Failed to kill container: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SandboxError::docker(&format!(
                "Failed to kill container {}: {}",
                name, stderr
            )));
        }

        Ok(())
    }

    pub fn list_containers(&self) -> Result<Vec<String>, SandboxError> {
        let output = Command::new(&self.docker_cmd)
            .args(["ps", "-q"])
            .output()
            .map_err(|e| SandboxError::docker(&format!("Failed to list containers: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SandboxError::docker(&format!(
                "Failed to list containers: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<String> = stdout
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(containers)
    }
}

impl Default for DockerClient {
    fn default() -> Self {
        Self::new()
    }
}
