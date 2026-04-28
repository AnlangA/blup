# AGENTS.md

## Purpose

`sandboxes/` contains isolated execution environments for deterministic tools: code compilation, code execution, math engines, grading, and document compilation.

## Scope

### Phase 1 deliverables

None. Phase 1 does not run user code and does not require a sandbox.

### Phase 2 deliverables

- Docker-based sandbox for code execution and assessment.
- Resource limits, disabled network by default, and structured audit logs.
- Tool request and response schemas coordinated with `schemas/` and `crates/`.

### Phase 2.5 deliverables

- Controlled Typst compilation environment for learning-document exports.
- Import helper tools only if they are isolated from private user data leaks.

### Future evaluation

WASI and Firecracker microVMs may be evaluated after Docker-based isolation is well specified and tested.

## Module Responsibilities

- Execute deterministic tasks for Core through structured requests.
- Enforce timeout, CPU, memory, disk, process, network, input, and output limits.
- Return real execution results, never fabricated success.
- Emit structured logs for audit and debugging.

## Resource Limits

Initial Phase 2 defaults:

| Resource | Limit | Notes |
| --- | --- | --- |
| Compile timeout | 30 seconds | terminate after timeout |
| Run timeout | 10 seconds | terminate after timeout |
| Memory | 512 MB | prevent exhaustion attacks |
| CPU | 1 core | limit abuse |
| Disk | 100 MB temporary space | prevent unbounded output |
| Network | disabled by default | enable only with explicit approval and audit |
| Processes | maximum 10 | reduce fork-bomb risk |

## Testing and Quality Gates

- Timeout tests.
- Memory and CPU limit tests.
- Disk limit and output truncation tests.
- Disabled-network tests.
- Malicious input tests.
- Non-zero exit and compiler-error tests.
- Typst compile success and failure tests in Phase 2.5.

## Logging and Observability

Sandbox audit logs should include:

```text
request_id, session_id, tool_kind, image_id, limits, exit_code, timed_out, duration_ms, stdout_truncated, stderr_truncated, resource_usage
```

Do not log full private source documents, API keys, or unbounded stdout/stderr.

## Security and Privacy Rules

- Never run user-submitted code on the host.
- Never mount sensitive host directories.
- Disable network by default.
- Treat all inputs as untrusted.
- Fail closed when limits cannot be enforced.

## Do Not

- Do not silently swallow sandbox errors.
- Do not fake successful execution.
- Do not add sandbox functionality to Phase 1.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../schemas/AGENTS.md`](../schemas/AGENTS.md)
- [`../crates/AGENTS.md`](../crates/AGENTS.md)
- [`../tests/AGENTS.md`](../tests/AGENTS.md)
