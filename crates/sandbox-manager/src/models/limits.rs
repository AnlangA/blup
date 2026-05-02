use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxLimits {
    pub compile_timeout_secs: u64,
    pub run_timeout_secs: u64,
    pub memory_mb: u64,
    pub cpu_count: f64,
    pub disk_mb: u64,
    pub network_enabled: bool,
    pub max_processes: u32,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            compile_timeout_secs: 30,
            run_timeout_secs: 10,
            memory_mb: 512,
            cpu_count: 1.0,
            disk_mb: 100,
            network_enabled: false,
            max_processes: 10,
        }
    }
}

impl SandboxLimits {
    pub fn strict() -> Self {
        Self {
            compile_timeout_secs: 10,
            run_timeout_secs: 5,
            memory_mb: 256,
            cpu_count: 0.5,
            disk_mb: 50,
            network_enabled: false,
            max_processes: 5,
        }
    }

    pub fn relaxed() -> Self {
        Self {
            compile_timeout_secs: 60,
            run_timeout_secs: 30,
            memory_mb: 1024,
            cpu_count: 2.0,
            disk_mb: 500,
            network_enabled: false,
            max_processes: 20,
        }
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

    pub fn with_timeouts(mut self, compile: u64, run: u64) -> Self {
        self.compile_timeout_secs = compile;
        self.run_timeout_secs = run;
        self
    }
}
