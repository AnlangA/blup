# AGENTS.md

## Purpose

`plugins/` is reserved for domain-specific learning extensions. A plugin may provide capabilities for programming, mathematics, language learning, physics simulation, art, or other specialized learning domains.

## Scope

### Phase 1 deliverables

None. The plugin system is not part of Phase 1.

### Future deliverables

- Plugin manifests with metadata, capabilities, permissions, input schemas, and output schemas.
- Structured plugin communication over HTTP microservices or stdin/stdout first, with possible migration to WASM Component Model when the project is ready.
- Plugin contract tests and lifecycle management.

## Module Responsibilities

- Provide domain capabilities without becoming the system controller.
- Produce structured `LessonSpec`, `AssessmentSpec`, `SceneSpec`, or `ToolRequest` outputs.
- Request privileged actions through Core and Tool Router, never directly.
- Be loadable, unloadable, auditable, and permission-limited.

## Permission Model

Potential future permissions:

```text
read:curriculum
read:user_profile
generate:content
generate:assessment
generate:scene
request:tool
tool:math
tool:code_run
tool:render
```

Actual manifests must use English permission names only. User profile access must be minimized and redacted.

Forbidden capabilities:

- Direct file-system access.
- Direct network access.
- Direct shell command execution.
- Direct database access.
- Direct access to other plugins.
- Direct manipulation of Bevy ECS internals.

## Lifecycle

```text
Load → Init → Activate → Execute → Pause → Unload
```

Every lifecycle step must have permission checks and structured audit logs.

## Testing and Quality Gates

- Manifest schema validation.
- Contract tests for each declared capability.
- Permission-denial tests.
- Malformed input tests.
- Lifecycle tests for load, failure, pause, and unload.

## Logging and Observability

Plugin logs must include plugin ID, version, request ID, capability, permission decision, duration, and structured error codes. Do not log raw private learner content unless explicitly redacted and allowed.

## Security and Privacy Rules

- Plugins are untrusted by default.
- Plugins must not decide final learning progress directly.
- Plugins must not treat LLM output as deterministic computation.
- Plugins must not access secrets, network, files, databases, or other plugins directly.

## Do Not

- Do not implement plugin functionality in Phase 1.
- Do not bypass Core for tool execution.
- Do not depend on unstable WASM Component Model details until an ADR accepts that risk.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../schemas/AGENTS.md`](../schemas/AGENTS.md)
- [`../crates/AGENTS.md`](../crates/AGENTS.md)
