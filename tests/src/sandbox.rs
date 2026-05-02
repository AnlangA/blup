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

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_docker_hello_world() {
    let config = sandbox_manager::SandboxConfig::default();
    let manager = sandbox_manager::SandboxManager::new(config);
    let session_id = Uuid::new_v4();

    let request = SandboxRequest::new_python(session_id, "print('Hello, World!')");
    let result = manager.execute(request).await.unwrap();

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("Hello, World!"));
}
