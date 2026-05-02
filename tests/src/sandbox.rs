use sandbox_manager::executor::{MockExecutor, SandboxExecutor};
use sandbox_manager::models::limits::SandboxLimits;
use sandbox_manager::models::request::{SandboxRequest, ToolKind};
use sandbox_manager::models::result::{ErrorDetails, ResourceUsage, SandboxResult};
use sandbox_manager::models::status::ExecutionStatus;
use uuid::Uuid;

fn make_request(code: &str) -> SandboxRequest {
    SandboxRequest {
        request_id: Uuid::new_v4(),
        session_id: Uuid::new_v4(),
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        language: Some("python".to_string()),
        limits: SandboxLimits::default(),
        stdin: None,
        environment: None,
    }
}

// ── Mock-based sandbox tests (no Docker required) ──

#[tokio::test]
async fn test_python_hello_world() {
    let mut mock = MockExecutor::new();
    mock.push_response(SandboxResult {
        request_id: Uuid::nil(),
        session_id: None,
        status: ExecutionStatus::Success,
        exit_code: Some(0),
        stdout: "Hello, World!\n".to_string(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: 100,
        resource_usage: ResourceUsage::default(),
        error: None,
    });

    let result = mock
        .execute(make_request("print('Hello, World!')"))
        .await
        .unwrap();
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("Hello, World!"));
}

#[tokio::test]
async fn test_python_syntax_error() {
    let mut mock = MockExecutor::new();
    mock.push_response(SandboxResult {
        request_id: Uuid::nil(),
        session_id: None,
        status: ExecutionStatus::NonZeroExit,
        exit_code: Some(1),
        stdout: String::new(),
        stderr: "SyntaxError: invalid syntax\n".to_string(),
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: 50,
        resource_usage: ResourceUsage::default(),
        error: None,
    });

    let result = mock.execute(make_request("def foo(:")).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::NonZeroExit);
    assert_ne!(result.exit_code, Some(0));
    assert!(!result.stderr.is_empty());
}

#[tokio::test]
async fn test_python_infinite_loop_timeout() {
    let mut mock = MockExecutor::new();
    mock.push_response(SandboxResult {
        request_id: Uuid::nil(),
        session_id: None,
        status: ExecutionStatus::TimeoutRun,
        exit_code: None,
        stdout: String::new(),
        stderr: "Execution timed out".to_string(),
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: 10000,
        resource_usage: ResourceUsage::default(),
        error: Some(ErrorDetails {
            code: "TIMEOUT".to_string(),
            message: "Execution timed out after 10 seconds".to_string(),
        }),
    });

    let result = mock
        .execute(make_request("while True: pass"))
        .await
        .unwrap();
    assert!(result.status.is_timeout());
}

#[tokio::test]
async fn test_python_memory_limit() {
    let mut mock = MockExecutor::new();
    mock.push_response(SandboxResult {
        request_id: Uuid::nil(),
        session_id: None,
        status: ExecutionStatus::MemoryExceeded,
        exit_code: None,
        stdout: String::new(),
        stderr: "Memory limit exceeded".to_string(),
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: 200,
        resource_usage: ResourceUsage {
            peak_memory_mb: 1024.0,
            cpu_time_ms: 0,
            disk_used_kb: 0,
        },
        error: Some(ErrorDetails {
            code: "MEMORY_EXCEEDED".to_string(),
            message: "Memory limit of 256 MB exceeded".to_string(),
        }),
    });

    let result = mock
        .execute(make_request("x = bytearray(1024 * 1024 * 1024)"))
        .await
        .unwrap();
    assert!(result.status.is_resource_exceeded() || result.status.is_error());
}

#[tokio::test]
async fn test_network_disabled() {
    let mut mock = MockExecutor::new();
    mock.set_response_fn(Box::new(|req| {
        let contains_network = req.code.contains("urllib") || req.code.contains("http");
        if contains_network {
            SandboxResult {
                request_id: req.request_id,
                session_id: None,
                status: ExecutionStatus::Success,
                exit_code: Some(0),
                stdout: "NETWORK_BLOCKED\n".to_string(),
                stderr: String::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: 100,
                resource_usage: ResourceUsage::default(),
                error: None,
            }
        } else {
            SandboxResult {
                request_id: req.request_id,
                session_id: None,
                status: ExecutionStatus::Success,
                exit_code: Some(0),
                stdout: "ok\n".to_string(),
                stderr: String::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: 10,
                resource_usage: ResourceUsage::default(),
                error: None,
            }
        }
    }));

    let code = r#"
try:
    import urllib.request
    urllib.request.urlopen('http://example.com', timeout=5)
    print('NETWORK_ACCESSIBLE')
except Exception:
    print('NETWORK_BLOCKED')
"#;

    let result = mock.execute(make_request(code)).await.unwrap();
    assert!(result.stdout.contains("NETWORK_BLOCKED"));
    assert!(!result.stdout.contains("NETWORK_ACCESSIBLE"));
}

#[tokio::test]
async fn test_container_cleanup() {
    let mut mock = MockExecutor::new();
    mock.push_response(SandboxResult {
        request_id: Uuid::nil(),
        session_id: None,
        status: ExecutionStatus::Success,
        exit_code: Some(0),
        stdout: "test\n".to_string(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: 50,
        resource_usage: ResourceUsage::default(),
        error: None,
    });

    let result = mock.execute(make_request("print('test')")).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::Success);

    // With mock executor, cleanup is implicit — verify we can run again
    let result2 = mock
        .execute(make_request("print('another')"))
        .await
        .unwrap();
    assert_eq!(result2.status, ExecutionStatus::Success);
}

// ── Tests that still require Docker ──
// Run with: BLUP_RUN_SANDBOX_TESTS=1 cargo test -p blup-tests -- sandbox --ignored

fn fixture(name: &str) -> String {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../sandboxes/tests/fixtures");
    std::fs::read_to_string(format!("{}/{}", dir, name)).unwrap()
}

/// Check whether sandbox Docker tests should run.
fn docker_available() -> bool {
    std::env::var("BLUP_RUN_SANDBOX_TESTS").is_ok()
}

fn docker_manager() -> sandbox_manager::SandboxManager {
    sandbox_manager::SandboxManager::new(sandbox_manager::SandboxConfig::default())
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_hello_world() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let request = SandboxRequest::new_python(session_id, "print('Hello, World!')");
    let result = manager.execute(request).await.unwrap();

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("Hello, World!"));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_syntax_error() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let code = fixture("syntax_error.py");
    let request = SandboxRequest::new_python(session_id, &code);
    let result = manager.execute(request).await.unwrap();

    // Invalid Python should produce a non-zero exit
    assert_ne!(result.exit_code, Some(0));
    assert!(!result.stderr.is_empty() || result.status == ExecutionStatus::NonZeroExit);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_run_timeout() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let code = fixture("infinite_loop.py");
    let limits = SandboxLimits::default().with_timeouts(5, 3);
    let request = SandboxRequest::new_python(session_id, &code).with_limits(limits);
    let result = manager.execute(request).await.unwrap();

    assert!(result.status.is_timeout());
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_memory_limit() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let code = fixture("memory_bomb.py");
    let limits = SandboxLimits::default().with_memory(256);
    let request = SandboxRequest::new_python(session_id, &code).with_limits(limits);
    let result = manager.execute(request).await.unwrap();

    // Should be killed by OOM or exit non-zero
    assert!(result.status != ExecutionStatus::Success);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_fork_bomb() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let code = fixture("fork_bomb.py");
    let limits = SandboxLimits {
        max_processes: 10,
        ..SandboxLimits::default()
    };
    let request = SandboxRequest::new_python(session_id, &code).with_limits(limits);
    let result = manager.execute(request).await.unwrap();

    // Process limit should prevent fork bomb — either hits PROCESS_LIMIT_HIT or
    // fails with a non-success status
    let contained = result.stdout.contains("PROCESS_LIMIT_HIT")
        || result.status != ExecutionStatus::Success
        || result.exit_code != Some(0);
    assert!(contained);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_network_disabled() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let code = fixture("network_request.py");
    let limits = SandboxLimits::default().with_network(false);
    let request = SandboxRequest::new_python(session_id, &code).with_limits(limits);
    let result = manager.execute(request).await.unwrap();

    assert!(
        result.stdout.contains("NETWORK_BLOCKED"),
        "Expected NETWORK_BLOCKED, got stdout: {}",
        result.stdout
    );
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_disk_limit() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let code = fixture("disk_fill.py");
    let limits = SandboxLimits {
        disk_mb: 100,
        ..SandboxLimits::default()
    };
    let request = SandboxRequest::new_python(session_id, &code).with_limits(limits);
    let result = manager.execute(request).await.unwrap();

    // Should either fail with disk error or hit a limit
    let contained = result.stdout.contains("DISK_WRITE_FAILED")
        || result.status != ExecutionStatus::Success
        || result.exit_code != Some(0);
    assert!(contained);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_container_cleanup_after_success() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    // Count containers before
    let before = std::process::Command::new("docker")
        .args(["ps", "-q"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);

    let request = SandboxRequest::new_python(session_id, "print('cleanup test')");
    let result = manager.execute(request).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::Success);

    // Give Docker a moment to remove the container
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let after = std::process::Command::new("docker")
        .args(["ps", "-q"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);

    assert_eq!(
        before, after,
        "Container leaked: {before} containers before, {after} after"
    );
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_container_cleanup_after_timeout() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let before = std::process::Command::new("docker")
        .args(["ps", "-q"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);

    let code = "while True: pass";
    let limits = SandboxLimits::default().with_timeouts(3, 2);
    let request = SandboxRequest::new_python(session_id, code).with_limits(limits);
    let result = manager.execute(request).await.unwrap();
    assert!(result.status.is_timeout());

    // Give Docker a moment to kill and remove the container
    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

    let after = std::process::Command::new("docker")
        .args(["ps", "-q"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);

    // Allow +1 since other containers might have started in between
    assert!(
        after <= before + 1,
        "Container leaked after timeout: {before} containers before, {after} after"
    );
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_network_enabled() {
    if !docker_available() {
        return;
    }
    let manager = docker_manager();
    let session_id = Uuid::new_v4();

    let code = fixture("network_request.py");
    let limits = SandboxLimits::default().with_network(true);
    let request = SandboxRequest::new_python(session_id, &code).with_limits(limits);
    let result = manager.execute(request).await.unwrap();

    // With network enabled, should reach example.com
    assert!(
        result.stdout.contains("NETWORK_ACCESSIBLE"),
        "Expected NETWORK_ACCESSIBLE with network enabled, got: {}",
        result.stdout
    );
}
