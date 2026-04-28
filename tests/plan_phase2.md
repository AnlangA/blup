# Tests Module — Phase 2: Sandbox Security and Assessment Engine Tests

## Module Overview

Phase 2 adds tests for the sandbox execution layer and the assessment engine. These tests verify resource limits are actually enforced (not just configured), malicious inputs are contained, and assessment scoring is correct and deterministic.

## Phase 2 Test Scope

| Test Category | Purpose | Coverage Target |
|---------------|---------|-----------------|
| Sandbox timeout tests | Verify compile and run timeouts are enforced | All tool kinds |
| Sandbox resource limit tests | Verify memory, CPU, disk, process limits | All limit types |
| Sandbox network isolation tests | Verify network is actually disabled | All sandbox types |
| Sandbox malicious input tests | Verify sandbox survives attacks | Fork bombs, memory bombs, /dev attacks |
| Sandbox cleanup tests | Verify containers are always destroyed | Success and error paths |
| Assessment engine tests | Verify exercise generation and answer evaluation | All exercise types |
| LLM gateway tests | Provider mapping, retry, rate limiting | OpenAI + Anthropic |
| Storage tests | CRUD, migrations, concurrent access | All models |

## File Structure

```
tests/
├── sandbox/
│   ├── mod.rs
│   ├── timeout_tests.rs          # Compile and run timeouts
│   ├── memory_limit_tests.rs     # Memory exhaustion
│   ├── cpu_limit_tests.rs        # CPU-bound attacks
│   ├── disk_limit_tests.rs       # Disk exhaustion
│   ├── network_disabled_tests.rs # Network is unreachable
│   ├── process_limit_tests.rs    # Fork bombs
│   ├── malicious_input_tests.rs  # /dev/null flood, /proc enumeration
│   ├── exit_code_tests.rs        # Non-zero exits, signals
│   ├── cleanup_tests.rs          # Container always destroyed
│   ├── typst_compile_tests.rs    # Typst compilation success + failure
│   └── fixtures/
│       ├── infinite_loop.py
│       ├── memory_bomb.py
│       ├── fork_bomb.py
│       ├── network_request.py
│       ├── disk_fill.py
│       ├── valid_hello.py
│       └── syntax_error.py
├── assessment/
│   ├── mod.rs
│   ├── exercise_generation_test.rs
│   ├── multiple_choice_eval_test.rs
│   ├── short_answer_eval_test.rs
│   ├── coding_eval_test.rs
│   ├── reflection_eval_test.rs
│   └── rubric_test.rs
├── llm_gateway/                     # Python gateway tests
│   ├── test_openai_provider.py
│   ├── test_anthropic_provider.py
│   ├── test_retry.py
│   ├── test_rate_limiter.py
│   └── test_caching.py
└── storage/
    ├── mod.rs
    ├── migration_test.rs
    ├── session_crud_test.rs
    ├── curriculum_crud_test.rs
    ├── progress_test.rs
    ├── message_crud_test.rs
    └── concurrent_access_test.rs
```

## Sandbox Security Tests

### Test Requirements

- **Docker daemon must be available** in the CI environment.
- Tests are gated behind a `#[cfg(feature = "sandbox-tests")]` or environment variable (`BLUP_RUN_SANDBOX_TESTS=1`).
- All sandbox tests use **synthetic malicious inputs only** — no real learner code.
- Container cleanup is verified even when tests fail (using `Drop` guards).

### Timeout Tests

```rust
// sandbox/timeout_tests.rs (conceptual)
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_compile_timeout_kills_container() {
    // Python code that hangs at import time (simulated compile timeout)
    let code = "import time; time.sleep(60)";
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        limits: SandboxLimits {
            compile_timeout_secs: 5,
            ..default()
        },
    };

    let result = sandbox.execute(request).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::TimeoutCompile);
    assert!(result.duration_ms < 7000); // With some tolerance
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_run_timeout_kills_infinite_loop() {
    let code = "while True: pass";
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        limits: SandboxLimits {
            run_timeout_secs: 3,
            ..default()
        },
    };

    let result = sandbox.execute(request).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::TimeoutRun);
}
```

### Resource Limit Tests

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_memory_limit_kills_allocation_bomb() {
    let code = "x = bytearray(1024 * 1024 * 1024)"; // 1GB allocation
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        limits: SandboxLimits {
            memory_mb: 256,
            ..default()
        },
    };

    let result = sandbox.execute(request).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::MemoryExceeded);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_cpu_limit_prevents_multi_core_abuse() {
    let code = r#"
import multiprocessing as mp
def burn():
    while True: pass
with mp.Pool(4) as p:
    p.map(burn, range(4))
"#;
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        limits: SandboxLimits { cpu_count: 1.0, ..default() },
    };

    let result = sandbox.execute(request).await.unwrap();
    // CPU usage should be capped at ~1 core worth
    assert!(result.resource_usage.cpu_time_ms < result.duration_ms * 2);
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_disk_limit_prevents_big_writes() {
    let code = "with open('/tmp/big', 'wb') as f: f.write(b'0' * 500 * 1024 * 1024)"; // 500MB
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        limits: SandboxLimits { disk_mb: 100, ..default() },
    };

    let result = sandbox.execute(request).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::DiskExceeded);
}
```

### Network Isolation Tests

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_network_is_disabled_by_default() {
    let code = r#"
try:
    import urllib.request
    urllib.request.urlopen('http://example.com', timeout=5)
    print('NETWORK_ACCESSIBLE')
except Exception:
    print('NETWORK_BLOCKED')
"#;
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        limits: SandboxLimits { network_enabled: false, ..default() },
    };

    let result = sandbox.execute(request).await.unwrap();
    assert!(result.stdout.contains("NETWORK_BLOCKED"));
    // NOT "NETWORK_ACCESSIBLE"
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_network_can_be_enabled_when_approved() {
    // With network enabled, should be able to reach external hosts
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: "import urllib.request; urllib.request.urlopen('http://example.com', timeout=10)".into(),
        limits: SandboxLimits { network_enabled: true, ..default() },
    };

    let result = sandbox.execute(request).await.unwrap();
    assert_eq!(result.status, ExecutionStatus::Success);
}
```

### Malicious Input Tests

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_fork_bomb_is_contained() {
    let code = r#"
import os
while True:
    os.fork()
"#;
    let request = SandboxRequest {
        tool_kind: ToolKind::PythonExec,
        code: code.to_string(),
        limits: SandboxLimits { max_processes: 10, ..default() },
    };

    let result = sandbox.execute(request).await.unwrap();
    // Should hit process limit, not crash the host
    assert!(result.status == ExecutionStatus::ResourceExceeded
            || result.exit_code != Some(0));
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_dev_null_flood_is_contained() {
    let code = r#"
while True:
    with open('/dev/null', 'w') as f:
        f.write('x' * 1024 * 1024)
"#;
    // Should hit disk limit or timeout, not fill host disk
    let result = sandbox.execute(request).await.unwrap();
    assert!(result.status != ExecutionStatus::Success);
}
```

### Cleanup Tests

```rust
#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_is_destroyed_after_success() {
    let containers_before = docker_ps_count().await;
    sandbox.execute(valid_python_request()).await.unwrap();
    let containers_after = docker_ps_count().await;
    assert_eq!(containers_before, containers_after, "Container was not cleaned up");
}

#[tokio::test]
#[ignore = "requires Docker"]
async fn test_container_is_destroyed_after_error() {
    let containers_before = docker_ps_count().await;
    let _ = sandbox.execute(timeout_request()).await; // Will timeout
    let containers_after = docker_ps_count().await;
    assert_eq!(containers_before, containers_after, "Container was not cleaned up after error");
}
```

## Assessment Engine Tests

```rust
// assessment/multiple_choice_eval_test.rs (conceptual)
#[tokio::test]
async fn test_multiple_choice_correct_answer_scores_full() {
    let exercise = Exercise {
        exercise_type: ExerciseType::MultipleChoice {
            options: vec!["A".into(), "B".into(), "C".into()],
            correct_index: 1,
        },
        max_score: 1.0,
        ..default()
    };

    let answer = json!({"selected_index": 1});
    let result = engine.evaluate(&exercise, &answer, &llm, None).await.unwrap();

    assert_eq!(result.score, 1.0);
    assert!(result.is_correct);
}

#[tokio::test]
async fn test_multiple_choice_wrong_answer_scores_zero() {
    let answer = json!({"selected_index": 0}); // Wrong
    let result = engine.evaluate(&exercise, &answer, &llm, None).await.unwrap();
    assert_eq!(result.score, 0.0);
    assert!(!result.is_correct);
}

#[tokio::test]
async fn test_multiple_choice_evaluation_is_deterministic() {
    // Run 100 times, expect same result every time
    for _ in 0..100 {
        let result = engine.evaluate(&exercise, &answer, &llm, None).await.unwrap();
        assert_eq!(result.score, 1.0);
    }
}

#[tokio::test]
async fn test_coding_evaluation_delegates_to_sandbox() {
    let exercise = Exercise {
        exercise_type: ExerciseType::Coding {
            language: "python".into(),
            test_cases: vec![
                TestCase { input: "2, 3".into(), expected_output: "5".into() },
                TestCase { input: "-1, 1".into(), expected_output: "0".into() },
            ],
        },
        max_score: 2.0,
        ..default()
    };

    let answer = json!({"code": "def add(a, b): return a + b"});
    let result = engine.evaluate(&exercise, &answer, &llm, Some(&sandbox)).await.unwrap();

    assert_eq!(result.score, 2.0); // Both test cases pass
}
```

## LLM Gateway Tests (Python)

These tests run against the Python LLM Gateway service using `pytest` and `httpx` for async HTTP testing. They mock the `openai` and `anthropic` SDK calls to avoid real API usage.

```python
# test_openai_provider.py (conceptual)
import pytest
from unittest.mock import AsyncMock, patch
from src.providers.openai_provider import OpenAIProvider

@pytest.mark.asyncio
async def test_openai_provider_maps_request_correctly():
    with patch("openai.AsyncOpenAI") as mock_client:
        mock_client.return_value.chat.completions.create = AsyncMock(
            return_value=mock_openai_response()
        )
        provider = OpenAIProvider(api_key="test-key")
        response = await provider.complete(GatewayRequest(
            model="gpt-4o",
            messages=[{"role": "user", "content": "Hello"}],
            max_tokens=100,
        ))
        assert response.provider == "openai"
        assert response.content == "Expected response"
        assert response.usage["total_tokens"] > 0

@pytest.mark.asyncio
async def test_anthropic_provider_maps_system_to_top_level():
    with patch("anthropic.AsyncAnthropic") as mock_client:
        provider = AnthropicProvider(api_key="test-key")
        response = await provider.complete(GatewayRequest(
            model="claude-sonnet-4-6",
            messages=[
                {"role": "system", "content": "You are a tutor."},
                {"role": "user", "content": "Hello"},
            ],
            max_tokens=100,
        ))
        # Verify system message is NOT in the messages list (moved to top-level)
        # The mock verifies the Anthropic SDK was called correctly
        call_kwargs = mock_client.return_value.messages.create.call_args.kwargs
        assert "system" in call_kwargs
        assert call_kwargs["system"] == "You are a tutor."
        assert len(call_kwargs["messages"]) == 1  # Only user message

@pytest.mark.asyncio
async def test_retry_on_429_with_backoff():
    with patch("openai.AsyncOpenAI") as mock_client:
        mock_client.return_value.chat.completions.create = AsyncMock(
            side_effect=[RateLimitError("429"), mock_openai_response()]
        )
        provider = OpenAIProvider(api_key="test-key")
        response = await provider.complete(test_request())
        # Second call succeeded after retry
        assert response.provider == "openai"
        assert mock_client.return_value.chat.completions.create.call_count == 2

@pytest.mark.asyncio
async def test_circuit_breaker_opens_after_failures():
    breaker = CircuitBreaker(failure_threshold=3)
    with patch("openai.AsyncOpenAI") as mock_client:
        mock_client.return_value.chat.completions.create = AsyncMock(
            side_effect=ProviderUnavailableError("503")
        )
        provider = OpenAIProvider(api_key="test-key")
        for _ in range(3):
            with pytest.raises(ProviderUnavailableError):
                await breaker.call("openai", provider.complete, test_request())
        # 4th call should fail immediately with CircuitBreakerOpenError
        with pytest.raises(CircuitBreakerOpenError):
            await breaker.call("openai", provider.complete, test_request())
```

## Storage Tests

```rust
// storage/migration_test.rs (conceptual)
#[tokio::test]
async fn test_migrations_run_forward() {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("../crates/storage/src/migrations")
        .run(&pool)
        .await
        .unwrap();
    // Verify tables exist
}

#[tokio::test]
async fn test_migrations_are_idempotent() {
    // Run migrations twice; second run should be no-op
}

#[tokio::test]
async fn test_session_crud_lifecycle() {
    let storage = Storage::connect("sqlite::memory:").await.unwrap();
    storage.run_migrations().await.unwrap();

    // Create
    let session = storage.create_session().await.unwrap();
    assert_eq!(session.state, SessionState::Idle);

    // Read
    let retrieved = storage.get_session(session.id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, session.id);

    // Update
    storage.update_session_state(session.id, SessionState::GoalInput).await.unwrap();
    let updated = storage.get_session(session.id).await.unwrap().unwrap();
    assert_eq!(updated.state, SessionState::GoalInput);

    // Delete
    storage.delete_session(session.id).await.unwrap();
    assert!(storage.get_session(session.id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_concurrent_session_updates_no_cross_talk() {
    // Create 10 sessions; update them concurrently
    // Verify each session's state is independent
}
```

## Coverage Targets

| Area | Phase 2 Target |
|------|----------------|
| Sandbox execution paths | ≥90% |
| Assessment engine paths | ≥85% |
| LLM gateway provider mapping | 100% |
| Storage CRUD operations | 100% |
| Sandbox security (malicious input) | All attack vectors |

## Quality Gates

- [ ] All sandbox resource limits are verified by tests (not just configured)
- [ ] Network is actually disabled (test proves it is unreachable)
- [ ] All malicious inputs are contained (sandbox survives; host unaffected)
- [ ] Containers are always cleaned up (test verifies no leaked containers)
- [ ] Assessment scoring is deterministic (same input = same score, 100 runs)
- [ ] Coding evaluation always delegates to sandbox (never runs code in-process)
- [ ] LLM gateway correctly maps to both OpenAI and Anthropic formats
- [ ] Storage migrations run forward and are idempotent
- [ ] All tests pass without external network access (except mock servers on localhost)
