# Sandboxes Module — Implementation Plan

## Module Overview

`sandboxes/` contains isolated execution environments for deterministic tools: code compilation, code execution, math engines, grading scripts, and document compilation. The sandbox layer is the boundary between trusted Core infrastructure and untrusted user-submitted code.

**Core principle:** Every deterministic operation that could be faked by an LLM must run through a real sandbox with observable, auditable results.

## Phase Scope

| Phase | Deliverables | Status |
|-------|-------------|--------|
| Phase 1 | None — Phase 1 does not run user code | Explicitly excluded |
| Phase 2 | Docker-based code execution sandbox, resource limits, disabled network, audit logs | Planned |
| Phase 2.5 | Typst compilation environment, import helper isolation | Planned |
| Future | WASI and Firecracker microVM evaluation | Deferred |

## Phase 2 Detailed Plan

### Architecture

```
┌──────────────────────────────────────────────┐
│               Agent Core (crates/)            │
│                      │                        │
│              SandboxRequest                    │
│              (JSON Schema)                    │
│                      ▼                        │
│  ┌───────────────────────────────────────┐    │
│  │          Sandbox Manager               │    │
│  │  ┌─────────────────────────────────┐  │    │
│  │  │  Docker API Client (bollard)    │  │    │
│  │  └─────────────┬───────────────────┘  │    │
│  │                │                       │    │
│  │  ┌─────────────▼───────────────────┐  │    │
│  │  │  Container Lifecycle            │  │    │
│  │  │  ┌──────────┐  ┌────────────┐   │  │    │
│  │  │  │  Create   │  │  Execute   │   │  │    │
│  │  │  │container  │──│  command   │   │  │    │
│  │  │  └──────────┘  └─────┬──────┘   │  │    │
│  │  │                      │           │  │    │
│  │  │  ┌──────────────────▼────────┐   │  │    │
│  │  │  │  Collect output, metrics  │   │  │    │
│  │  │  └──────────┬────────────────┘   │  │    │
│  │  │             │                     │  │    │
│  │  │  ┌──────────▼────────────────┐   │  │    │
│  │  │  │  Destroy container         │   │  │    │
│  │  │  └────────────────────────────┘   │  │    │
│  │  └───────────────────────────────────┘  │    │
│  └───────────────────────────────────────┘    │
│                      │                        │
│              SandboxResult                     │
│              (JSON Schema)                    │
└──────────────────────────────────────────────┘
```

### File Structure

```
sandboxes/
├── AGENTS.md
├── plan_phase2.md
├── docker/
│   ├── Dockerfile.python          # Python 3.12 + common packages
│   ├── Dockerfile.node            # Node.js 22 LTS
│   ├── Dockerfile.rust            # Rust toolchain
│   ├── Dockerfile.typst           # Typst compiler (Phase 2.5)
│   └── Dockerfile.math            # Math engines (SageMath, SymPy)
├── policies/
│   ├── seccomp-python.json        # Seccomp profile for Python
│   ├── seccomp-node.json          # Seccomp profile for Node.js
│   └── seccomp-default.json       # Default restrictive profile
├── tests/
│   ├── timeout_tests.rs
│   ├── memory_limit_tests.rs
│   ├── cpu_limit_tests.rs
│   ├── disk_limit_tests.rs
│   ├── network_disabled_tests.rs
│   ├── malicious_input_tests.rs
│   └── exit_code_tests.rs
└── README.md                      # Sandbox operator documentation
```

### Docker Images

#### Python Sandbox (`docker/Dockerfile.python`)

```dockerfile
FROM python:3.12-slim
RUN useradd -m sandbox -u 1000
RUN pip install --no-cache-dir sympy numpy
USER sandbox
WORKDIR /workspace
ENTRYPOINT ["python", "-c"]
```

#### Node.js Sandbox (`docker/Dockerfile.node`)

```dockerfile
FROM node:22-slim
RUN useradd -m sandbox -u 1000
USER sandbox
WORKDIR /workspace
ENTRYPOINT ["node", "-e"]
```

#### Math Sandbox (`docker/Dockerfile.math`)

```dockerfile
FROM python:3.12-slim
RUN useradd -m sandbox -u 1000 && \
    pip install --no-cache-dir sympy numpy scipy matplotlib
USER sandbox
WORKDIR /workspace
ENTRYPOINT ["python", "-c"]
```

### Resource Limits

Default Phase 2 limits, configurable per tool category:

| Resource | Limit | Enforcement | Notes |
|----------|-------|-------------|-------|
| Compile timeout | 30s | Docker `--stop-timeout` | Terminates container after timeout |
| Run timeout | 10s | Docker `--stop-timeout` | Separate from compile timeout |
| Memory | 512 MB | `--memory=512m` | OOM kill with structured error |
| CPU | 1 core | `--cpus=1` | Prevent multi-core abuse |
| Disk | 100 MB | `--storage-opt size=100M` | Prevent unbounded output |
| Network | disabled | `--network=none` | Enable only with explicit approval |
| Processes | 10 max | `--pids-limit=10` | Reduce fork-bomb risk |
| File descriptors | 64 max | `--ulimit nofile=64:64` | Prevent fd exhaustion |

### SandboxRequest Schema

Coordinated with `schemas/sandbox_request.v1.schema.json`:

```json
{
  "request_id": "uuid",
  "session_id": "uuid",
  "tool_kind": "python_exec | node_exec | rust_compile_run | math_eval | typst_compile",
  "code": "string (the source code or input to execute)",
  "language": "python | javascript | rust | latex | math",
  "limits": {
    "compile_timeout_secs": 30,
    "run_timeout_secs": 10,
    "memory_mb": 512,
    "cpu_count": 1,
    "disk_mb": 100,
    "network_enabled": false,
    "max_processes": 10
  },
  "stdin": "string (optional piped input)",
  "environment": { "key": "value" }
}
```

### SandboxResult Schema

```json
{
  "request_id": "uuid",
  "session_id": "uuid",
  "status": "success | timeout_compile | timeout_run | memory_exceeded | cpu_exceeded | disk_exceeded | non_zero_exit | network_blocked | internal_error",
  "exit_code": 0,
  "stdout": "truncated to 64KB",
  "stderr": "truncated to 64KB",
  "stdout_truncated": false,
  "stderr_truncated": false,
  "duration_ms": 1234,
  "resource_usage": {
    "peak_memory_mb": 45,
    "cpu_time_ms": 800,
    "disk_used_kb": 12
  },
  "error": {
    "code": "string",
    "message": "string (redacted, no internal paths)"
  }
}
```

### Implementation Steps (Phase 2)

1. **Docker verification**: Verify Docker daemon is available and accessible from the Rust server.
2. **Image build**: Build sandbox images from Dockerfiles; store image IDs for verification.
3. **Rust Docker client**: Integrate `bollard` crate (Rust Docker API client) or use `std::process::Command` to call `docker` CLI.
4. **Container lifecycle**:
   - Create container with resource limits and seccomp profile.
   - Attach to stdout/stderr streams.
   - Enforce timeouts at the Rust level (Tokio `timeout` wrapper around Docker API call).
   - Collect exit code, output, and resource metrics.
   - Destroy container (always, even on error — use `Drop` or `finally` pattern).
5. **Audit logging**: Emit structured logs per the logging contract.
6. **Error mapping**: Map Docker errors to typed `SandboxError` variants.

### Rust Crate Integration

The sandbox manager lives in `crates/agent-core` during Phase 2 (or a dedicated `crates/sandbox-manager` if complexity warrants):

```rust
// Conceptual API
use std::time::Duration;

struct SandboxConfig {
    image: String,
    compile_timeout: Duration,
    run_timeout: Duration,
    memory_mb: u64,
    cpu_count: f64,
    disk_mb: u64,
    network_enabled: bool,
    max_processes: u32,
}

struct SandboxRequest {
    request_id: Uuid,
    session_id: Uuid,
    tool_kind: ToolKind,
    code: String,
    stdin: Option<String>,
    config: SandboxConfig,
}

struct SandboxResult {
    request_id: Uuid,
    status: ExecutionStatus,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
    stdout_truncated: bool,
    stderr_truncated: bool,
    duration_ms: u64,
    resource_usage: ResourceUsage,
}

enum ExecutionStatus {
    Success,
    TimeoutCompile,
    TimeoutRun,
    MemoryExceeded,
    CpuExceeded,
    DiskExceeded,
    NonZeroExit,
    NetworkBlocked,
    InternalError,
}

trait SandboxManager {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError>;
    async fn health_check(&self) -> Result<(), SandboxError>;
    async fn image_info(&self) -> Vec<ImageInfo>;
}
```

### Container Creation (Detailed)

```rust
// sandbox/docker_executor.rs (conceptual)
use std::process::Command;
use tokio::time::timeout;

struct DockerExecutor {
    image: String,
    limits: SandboxLimits,
    seccomp_profile_path: PathBuf,
}

impl DockerExecutor {
    async fn execute(&self, request: SandboxRequest) -> Result<SandboxResult, SandboxError> {
        let container_name = format!("blup-sandbox-{}", request.request_id);

        // Build docker run command with ALL security flags
        let mut cmd = Command::new("docker");
        cmd.args(["run", "--rm"])                             // Auto-remove on exit
            .args(["--name", &container_name])
            // Resource limits
            .args(["--memory", &format!("{}m", self.limits.memory_mb)])
            .args(["--cpus", &format!("{}", self.limits.cpu_count)])
            .args(["--storage-opt", &format!("size={}m", self.limits.disk_mb)])
            .args(["--pids-limit", &format!("{}", self.limits.max_processes)])
            // Security hardening
            .args(["--security-opt", "no-new-privileges:true"])
            .args(["--security-opt", &format!("seccomp={}", self.seccomp_profile_path.display())])
            .args(["--read-only"])                            // Read-only root filesystem
            .args(["--tmpfs", "/workspace:rw,noexec,nosuid,size=100m"])  // Writable tmpfs
            .args(["--tmpfs", "/tmp:rw,noexec,nosuid,size=10m"])
            .args(["--cap-drop", "ALL"])                      // Drop all capabilities
            .args(["--cap-add", "NET_BIND_SERVICE"])           // Only if network enabled
            // Network
            .arg(if self.limits.network_enabled { "--network=bridge" } else { "--network=none" })
            // User
            .args(["--user", "1000:1000"])
            // Input
            .arg(self.image)
            .args(["sh", "-c", &request.code]);

        // Enforce timeout with Tokio
        let result = timeout(
            Duration::from_secs(self.limits.compile_timeout_secs + self.limits.run_timeout_secs),
            async {
                let output = tokio::task::spawn_blocking(move || cmd.output()).await??;
                Ok::<_, SandboxError>(output)
            }
        ).await;

        match result {
            Ok(Ok(output)) => {
                // ALWAYS verify container is removed, even on success
                self.force_remove_container(&container_name).await;
                self.parse_output(output)
            }
            Ok(Err(e)) => {
                self.force_remove_container(&container_name).await;
                Err(e)
            }
            Err(_elapsed) => {
                // Timeout: force-kill container
                self.force_kill_container(&container_name).await;
                Ok(SandboxResult {
                    status: ExecutionStatus::TimeoutRun,
                    duration_ms: (self.limits.compile_timeout_secs + self.limits.run_timeout_secs) * 1000,
                    ..Default::default()
                })
            }
        }
    }

    async fn force_remove_container(&self, name: &str) {
        let _ = Command::new("docker")
            .args(["rm", "-f", name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    async fn force_kill_container(&self, name: &str) {
        let _ = Command::new("docker")
            .args(["kill", "--signal=SIGKILL", name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        tokio::time::sleep(Duration::from_millis(500)).await;
        self.force_remove_container(name).await;
    }
}
```

### Seccomp Profile

A custom seccomp profile restricts system calls to the minimum needed for code execution:

```json
// sandboxes/policies/seccomp-python.json
{
  "defaultAction": "SCMP_ACT_ERRNO",
  "architectures": ["SCMP_ARCH_X86_64", "SCMP_ARCH_AARCH64"],
  "syscalls": [
    // ── Essential ──
    { "names": ["read", "write", "close", "exit", "exit_group"],
      "action": "SCMP_ACT_ALLOW" },
    { "names": ["fstat", "lseek", "mmap", "mprotect", "munmap", "brk"],
      "action": "SCMP_ACT_ALLOW" },

    // ── Memory allocation ──
    { "names": ["mremap", "madvise"],
      "action": "SCMP_ACT_ALLOW" },

    // ── File operations (tmpfs only) ──
    { "names": ["openat", "readlink", "getdents64", "newfstatat"],
      "action": "SCMP_ACT_ALLOW" },
    { "names": ["access", "faccessat"],
      "action": "SCMP_ACT_ALLOW" },

    // ── Process (limited) ──
    { "names": ["clone", "futex", "set_robust_list", "getpid", "tgkill"],
      "action": "SCMP_ACT_ALLOW" },
    { "names": ["rt_sigaction", "rt_sigprocmask", "rt_sigreturn",
                "sigaltstack", "prlimit64"],
      "action": "SCMP_ACT_ALLOW" },

    // ── Time ──
    { "names": ["clock_gettime", "clock_nanosleep", "nanosleep"],
      "action": "SCMP_ACT_ALLOW" },

    // ── Random (Python startup, crypto) ──
    { "names": ["getrandom", "getcwd"],
      "action": "SCMP_ACT_ALLOW" },

    // ── Thread-local storage ──
    { "names": ["set_tid_address", "arch_prctl", "sched_getaffinity"],
      "action": "SCMP_ACT_ALLOW" },

    // ── Resource limits ──
    { "names": ["setrlimit", "getrlimit", "rlimit_nproc"],
      "action": "SCMP_ACT_ALLOW" },

    // ── Read-only info (NOT leaked to host) ──
    { "names": ["uname", "readlinkat"],
      "action": "SCMP_ACT_ALLOW" }
  ]
}
```

**Explicitly blocked (defaultAction: ERRNO):**
- `fork`, `vfork` — prevented by pids-limit
- `execve`, `execveat` — no process spawning inside sandbox
- `mount`, `umount`, `pivot_root` — no filesystem manipulation
- `ptrace` — no process tracing
- `socket`, `bind`, `connect`, `sendto` — no network when disabled
- `chmod`, `chown`, `link`, `unlink`, `rename` — limited to tmpfs
- `reboot`, `shutdown`, `init_module`, `delete_module` — no kernel ops
- `personality` — prevented (can disable ASLR)
- `kcmp` — prevented (kernel pointer leak)

### Container Escape Prevention Checklist

| Vector | Mitigation | Verification |
|--------|-----------|-------------|
| Privileged container | Never use `--privileged` | CI test checks docker args |
| Host filesystem mount | Never use `-v /host/path:/container/path` | tmpfs only |
| Docker socket mount | Never mount `/var/run/docker.sock` | CI test |
| Capability abuse | `--cap-drop=ALL` | Verify `capsh --print` inside container |
| User namespace | `--user 1000:1000` (non-root) | Verify `id` inside container |
| Seccomp bypass | Custom restrictive profile | Malicious input tests |
| /proc leaks | `/proc` partially accessible but host info filtered | Test `/proc/self/mountinfo` |
| cgroup escape | Write access to cgroup dirs denied | Read-only rootfs + no CAP_SYS_ADMIN |
| Device access | `--device` flag never used | CI test checks docker args |
| PID namespace | Default Docker PID namespace | Test: `kill -9 1` from inside → container killed, not host |

### Audit Logging Implementation

```rust
// observability/sandbox_audit.rs (conceptual)
#[derive(Serialize)]
struct SandboxAuditLog {
    timestamp: chrono::DateTime<chrono::Utc>,
    request_id: Uuid,
    session_id: Uuid,
    tool_kind: String,
    image_id: String,
    image_tag: String,
    limits: SandboxLimitsLog,
    exit_code: Option<i32>,
    timed_out: bool,
    duration_ms: u64,
    stdout_size_bytes: usize,
    stderr_size_bytes: usize,
    stdout_truncated: bool,
    stderr_truncated: bool,
    resource_usage: ResourceUsageLog,
    container_cleaned_up: bool,
}

impl SandboxAuditLog {
    fn from_result(
        request: &SandboxRequest,
        result: &SandboxResult,
        image_tag: &str,
        container_cleaned_up: bool,
    ) -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            request_id: request.request_id,
            session_id: request.session_id,
            tool_kind: request.tool_kind.to_string(),
            image_id: request.image.clone(),
            image_tag: image_tag.to_string(),
            limits: SandboxLimitsLog {
                compile_timeout_secs: request.limits.compile_timeout_secs,
                run_timeout_secs: request.limits.run_timeout_secs,
                memory_mb: request.limits.memory_mb,
                cpu_count: request.limits.cpu_count,
                disk_mb: request.limits.disk_mb,
                network_enabled: request.limits.network_enabled,
                max_processes: request.limits.max_processes,
            },
            exit_code: result.exit_code,
            timed_out: matches!(result.status, ExecutionStatus::TimeoutCompile | ExecutionStatus::TimeoutRun),
            duration_ms: result.duration_ms,
            stdout_size_bytes: result.stdout.len(),
            stderr_size_bytes: result.stderr.len(),
            stdout_truncated: result.stdout_truncated,
            stderr_truncated: result.stderr_truncated,
            resource_usage: ResourceUsageLog {
                peak_memory_mb: result.resource_usage.peak_memory_mb,
                cpu_time_ms: result.resource_usage.cpu_time_ms,
                disk_used_kb: result.resource_usage.disk_used_kb,
            },
            container_cleaned_up,
        }
    }

    fn log(&self) {
        tracing::info!(
            target: "sandbox",
            request_id = %self.request_id,
            session_id = %self.session_id,
            tool_kind = %self.tool_kind,
            image_tag = %self.image_tag,
            exit_code = ?self.exit_code,
            timed_out = self.timed_out,
            duration_ms = self.duration_ms,
            memory_mb = self.resource_usage.peak_memory_mb,
            cpu_ms = self.resource_usage.cpu_time_ms,
            stdout_bytes = self.stdout_size_bytes,
            stderr_bytes = self.stderr_size_bytes,
            container_cleaned = self.container_cleaned_up,
            // NEVER log: stdout/stderr content, user code, API keys
        );
    }
}
```

### Phase 2.5 Additions: Typst Compilation

Typst compilation runs in its own sandbox with:
- **Image:** `Dockerfile.typst` based on a minimal image with `typst` CLI installed.
- **Input:** Typst source code + any referenced assets (fonts, images) copied into the container.
- **Output:** PDF binary + compilation log.
- **Resource limits:** Same defaults, with compile timeout extended to 60s for large documents.
- **Asset handling:** Assets are copied into the container, not mounted from the host. Output PDF is extracted via `docker cp` before container destruction.

**Security note on Typst:** Typst is designed to be safer than LaTeX (no shell escape), but we still sandbox it because user-provided Typst code could contain malicious package imports or exploit compiler bugs.

### Import Helper Isolation (Phase 2.5)

Import tools (PDF extraction, web fetching) run in sandboxed containers:
- Network access is **enabled** but restricted to the target URL (via HTTP proxy or iptables).
- Extracted content is validated and truncated before leaving the sandbox.
- Import containers never have access to local filesystem paths.

### WASI / Firecracker Evaluation (Future)

After Docker-based isolation is well-specified and tested in Phase 2:

| Technology | Pros | Cons | Evaluation Criteria |
|------------|------|------|---------------------|
| **Docker** | Mature, well-understood, rich tooling | Larger footprint, daemon dependency | Baseline |
| **gVisor** | Stronger isolation than Docker, OCI-compatible | Performance overhead, less mature | Sandbox escape resistance |
| **Firecracker** | Very strong isolation (microVM), fast boot | Requires KVM, heavier ops | Multi-tenant security |
| **WASI (Wasmtime)** | Lightweight, no daemon, cross-platform | Limited system call surface, immature ecosystem | Plugin sandboxing |

### Testing Strategy

| Test Category | Method | Tools |
|---------------|--------|-------|
| Timeout (compile) | Submit code with infinite loop at compile time | Docker container with 5s timeout |
| Timeout (run) | Submit code with `while True: pass` | Docker container with 3s timeout |
| Memory limit | Submit code that allocates 1GB | OOM kill assertion |
| CPU limit | Submit multi-threaded CPU-bound code | CPU usage metric assertion |
| Disk limit | Submit code that writes 500MB to disk | Disk exhaustion assertion |
| Network disabled | Submit code that tries `requests.get()` | Connection error assertion |
| Malicious input | Submit fork bomb, `/dev/null` flood, `/proc` enumeration | Resource exhaustion assertion |
| Non-zero exit | Submit code that calls `sys.exit(1)` | Exit code assertion |
| Compiler error | Submit invalid Python syntax | Structured error assertion |
| Valid execution | Submit correct Python that prints "hello" | Output assertion |
| Cleanup | Verify container is destroyed after each test | Docker `ps` assertion |

All sandbox tests must:
- Use synthetic inputs only (no real learner code).
- Run in CI (requires Docker in CI runner).
- Not depend on network access (except the network-disabled test, which verifies the block).
- Clean up containers even when tests fail.

### Logging and Observability

Audit logs for every sandbox execution:

```json
{
  "timestamp": "2025-01-01T00:00:00Z",
  "level": "INFO",
  "target": "sandbox",
  "request_id": "uuid",
  "session_id": "uuid",
  "tool_kind": "python_exec",
  "image_id": "sha256:abc123",
  "image_tag": "sandbox-python:v1",
  "limits": {
    "compile_timeout_secs": 30,
    "run_timeout_secs": 10,
    "memory_mb": 512,
    "network_enabled": false
  },
  "exit_code": 0,
  "timed_out": false,
  "duration_ms": 1234,
  "stdout_size_bytes": 256,
  "stderr_size_bytes": 0,
  "stdout_truncated": false,
  "stderr_truncated": false,
  "resource_usage": {
    "peak_memory_mb": 45,
    "cpu_time_ms": 800
  }
}
```

Never log: full stdout/stderr content (may contain private data), Docker host paths, API keys, or user code.

### Security and Privacy Rules

| Rule | Enforcement |
|------|-------------|
| Never run user code on the host | Only via Docker containers |
| Never mount sensitive host directories | Container has only `/workspace` tmpfs |
| Disable network by default | `--network=none` unless explicitly approved |
| Treat all inputs as untrusted | Seccomp profile + resource limits applied before execution |
| Fail closed | If limits can't be verified, refuse to execute |
| Destroy containers immediately | No container reuse between requests |
| Redact output in logs | Log only sizes and truncation flags, not content |

### Quality Gates

- [ ] All sandbox images build successfully and are versioned
- [ ] Resource limits are verified by tests (not just configured)
- [ ] Network is actually disabled (test proves it)
- [ ] All malicious input tests pass (sandbox survives)
- [ ] Container cleanup is guaranteed (no leaked containers)
- [ ] Audit logs include all required fields
- [ ] No host paths or credentials in logs or error messages
- [ ] Seccomp profiles are documented and reviewed

### Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Docker daemon unavailable in CI | Can't run sandbox tests | Mock Docker API for unit tests; integration tests require Docker label |
| Container escape vulnerability | Host compromise | Use seccomp, user namespaces, read-only rootfs; monitor Docker security advisories |
| Resource limit bypass | DoS on sandbox host | Test limits empirically; monitor resource usage anomalies |
| Image supply chain attack | Malicious code in sandbox image | Pin image digests; build images from verified base images; scan for vulnerabilities |
| Output smuggling (data exfiltration via stdout) | Information leak | Truncate output; don't log full output; treat output as untrusted |
| Typst package compromise (Phase 2.5) | Malicious Typst package in document | Disable Typst package download; use curated package cache only |
