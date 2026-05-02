# AGENTS.md

## Purpose

`crates/` contains Rust services and libraries. It is the trusted core for Agent orchestration, state transitions, LLM calls, validation, storage, tool routing, plugin hosting, and protocol implementation.

## Scope

### Phase 1 deliverables (completed)

- `crates/agent-core`: a single Rust crate containing:
  - Axum HTTP service.
  - Learning-flow state machine.
  - OpenAI-compatible LLM client boundary.
  - Prompt template loading.
  - Schema validation for structured LLM output and API payloads.
  - Structured logging with `tracing`.

### Phase 2 deliverables (completed)

- `crates/storage`: persistent storage crate with SQLite, SQLx migrations, backup/restore.
- `crates/assessment-engine`: exercise generation and deterministic evaluation (multiple choice, short answer, coding, reflection).
- `crates/sandbox-manager`: Docker-based sandboxed code execution with resource limits and mock executor support.
- `crates/blup-agent`: agent abstraction layer.

### Not yet in scope

- `llm-gateway` split-out Rust crate (currently a Python service at `services/llm-gateway/`).
- `tool-router`, `plugin-host`, or `bevy-protocol` crates (Phase 3).
- `content-pipeline` crate (Phase 2.5).
- Host execution of user-submitted code.

## Module Responsibilities

- Own the learning state machine and enforce valid transitions.
- Treat LLMs, plugins, sandboxes, importers, compilers, and renderers as external capabilities.
- Validate all structured inputs and outputs before changing state.
- Centralize error handling, authorization checks, redaction, and observability.

## Implementation Plan by Phase

- Phase 1: single `agent-core` crate for fast iteration. **Completed.**
- Phase 2: split `storage`, `assessment-engine`, and `sandbox-manager`. **Completed.**
- Phase 2.5: add content/import/export pipeline crates only after file permissions and artifact rules are defined.
- Phase 3: add `plugin-host`, `tool-router`, and `bevy-protocol` behind explicit schemas and permission checks.

## Commands

Expected repository-level Rust checks:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

If a crate adds feature flags, CI must test the feature combinations that affect public behavior.

## Testing and Quality Gates

- Unit tests for state transitions and error handling.
- Integration tests for REST and SSE endpoints.
- Mock LLM tests with deterministic responses.
- Schema validation tests for all API and LLM output boundaries.
- Retry tests for schema-validation failures and LLM transport failures.
- No default test may call a paid LLM API.

## Logging and Observability

Use `tracing` spans for requests, sessions, state transitions, LLM calls, validation, imports, exports, and tool execution.

Required fields where applicable:

```text
request_id, session_id, state, event, duration_ms, model, retry_count, validation_status, error_code
```

Redact prompts, imported private content, API keys, and user secrets. Log token counts and checksums instead of raw sensitive content.

## Security and Privacy Rules

- Core must not contain UI component code.
- Core must not execute user code on the host.
- LLM output is untrusted until validated.
- Tool, sandbox, plugin, and document compiler calls must go through explicit request types and permission checks.
- Errors returned to clients should be useful but must not expose secrets or internal stack traces.

## Do Not

- Do not use untyped string protocols where schemas or Rust types are required.
- Do not bypass validation to speed up a demo.
- Do not write API keys, local paths, or private learner records to logs.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../schemas/AGENTS.md`](../schemas/AGENTS.md)
- [`../prompts/AGENTS.md`](../prompts/AGENTS.md)
- [`../tests/AGENTS.md`](../tests/AGENTS.md)
