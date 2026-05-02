use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub docker_host: String,
    pub default_image: String,
    pub compile_timeout: Duration,
    pub run_timeout: Duration,
    pub memory_mb: u64,
    pub cpu_count: f64,
    pub disk_mb: u64,
    pub network_enabled: bool,
    pub max_processes: u32,
    pub seccomp_profile: Option<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            docker_host: "unix:///var/run/docker.sock".to_string(),
            default_image: "sandbox-python:latest".to_string(),
            compile_timeout: Duration::from_secs(30),
            run_timeout: Duration::from_secs(10),
            memory_mb: 512,
            cpu_count: 1.0,
            disk_mb: 100,
            network_enabled: false,
            max_processes: 10,
            seccomp_profile: None,
        }
    }
}

impl SandboxConfig {
    pub fn with_image(mut self, image: &str) -> Self {
        self.default_image = image.to_string();
        self
    }

    pub fn with_network(mut self, enabled: bool) -> Self {
        self.network_enabled = enabled;
        self
    }

    pub fn with_memory(mut self, mb: u64) -> Self {
        self.memory_mb = mb;
        self
    }

    pub fn with_cpu(mut self, count: f64) -> Self {
        self.cpu_count = count;
        self
    }

    pub fn with_timeouts(mut self, compile: Duration, run: Duration) -> Self {
        self.compile_timeout = compile;
        self.run_timeout = run;
        self
    }
}
