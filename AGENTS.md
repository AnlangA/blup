# AGENTS.md

## Purpose

Blup is an AI interactive learning-agent platform. A learner enters a learning goal, the system checks whether the goal is feasible, collects a learner profile, generates a personalized curriculum, and then teaches chapter by chapter with structured content, interaction, exercises, assessment, and feedback.

This file is the canonical planning and agent-instruction document for the repository. Nested `AGENTS.md` files add module-specific rules and must not contradict this file.

## Product Principles

- LLMs explain, plan, tutor, and draft. They must not pretend to execute deterministic work.
- Deterministic work must be delegated to real tools: math engines for calculation, sandboxes for code execution, validators for schemas, and compilers for document export.
- All important learning content, exercises, scene specs, imports, exports, and assessment results should be structured enough to validate, replay, and audit.
- Protocols come first. `schemas/` is the shared contract layer for apps, core services, plugins, sandboxes, and rendering.
- The Web UI renders the primary learning product. Bevy is an interactive rendering layer, not a replacement for the application UI.
- User privacy, learning records, imported materials, API keys, and local paths must never be committed or logged in raw form.

## Scope

### Current product direction

The long-term architecture is:

```text
Tauri desktop shell
├── Web UI              # chat, curriculum, chapter content, Markdown, formulas, code display
├── Rust Agent Core     # orchestration, state machine, LLM boundary, validation, tools
├── Storage             # sessions, progress, imported source metadata, generated artifacts
├── Sandbox layer       # real compilation, code execution, math tools, document compilers
├── Plugin system       # domain-specific learning capabilities behind permissions
└── Bevy renderer       # optional interactive 2D/3D/simulation scenes
```

### Explicit Phase 1 exclusions

Phase 1 must not include Tauri, Bevy, WASM plugins, Docker sandbox execution, database persistence, user authentication, payments, internationalization, or real execution of user-submitted code.

## Delivery Phases

| Phase | Goal | Main directories | Deliverable |
| --- | --- | --- | --- |
| Phase 0: Foundation | Make the repository buildable, checkable, and observable | root, `tools/`, `schemas/`, `docs-internal/` | bootstrap/check commands, schema validation, CI-quality policy, logging policy |
| Phase 1: Web learning assistant MVP | Single-user web learning flow | `schemas/`, `crates/agent-core`, `prompts/`, `apps/web-ui`, `tests/` | usable web assistant: goal feasibility → profile → curriculum → chapter teaching |
| Phase 2: Verification and persistence | Exercises, assessment, sandboxed execution, progress storage | `crates/storage`, `crates/assessment-engine`, `sandboxes/`, `tests/` | persistent learning sessions and deterministic assessment/tool results |
| Phase 2.5: Desktop and materials workflow | Desktop packaging, imports, Typst/PDF export | `apps/desktop`, `tools/`, `crates/content-pipeline` | local desktop app, source import, learning document export |
| Phase 3: Extensions and interactive scenes | Plugin host and Bevy scenes | `plugins/`, `crates/plugin-host`, `crates/tool-router`, `apps/bevy-viewer`, `assets/` | permissioned plugins and interactive learning scenes |

Each phase must produce a runnable, demonstrable product slice. Do not deliver only framework code or disconnected infrastructure.

## Phase 0: Repository Foundation

Phase 0 should be completed before or alongside early Phase 1 work.

### Required outcomes

- A documented bootstrap command that checks required local tooling by phase.
- A documented check command that runs formatters, linters, type checks, schema validation, and tests available in the repository.
- A schema validation path for all JSON Schema files and representative fixtures.
- A logging and redaction policy before adding LLM calls, imports, or sandbox execution.
- A CI plan that fails on formatting, lint, tests, schema errors, and accidental secrets.

### Planned commands

Use these names unless a later implementation has a strong reason to choose otherwise:

```text
scripts/bootstrap     # verify Rust, Node, package manager, and phase-specific tools
scripts/dev           # run the Phase 1 backend and frontend locally
scripts/check         # run all repository validators
scripts/schema-check  # validate schemas and fixtures
```

Do not hard-code personal paths, tokens, or machine-specific configuration in scripts.

## Phase 1 MVP Definition

### Deliverables

| Directory | Phase 1 deliverable |
| --- | --- |
| `schemas/` | JSON Schemas for `LearningGoal`, `FeasibilityResult`, `UserProfile`, `CurriculumPlan`, `Chapter`, `Message`, and `ChapterProgress` |
| `crates/agent-core` | Rust HTTP service using Axum, Tokio, Serde, reqwest, tracing, prompt loading, state machine, LLM boundary, schema validation |
| `prompts/` | Versioned prompt templates for feasibility checks, profile collection, curriculum planning, chapter teaching, and chapter Q&A |
| `apps/web-ui` | React or Svelte SPA with chat, curriculum sidebar, chapter content area, Markdown, KaTeX, and CodeMirror 6 |
| `tests/` | Integration tests for the core learning flow, HTTP API behavior, SSE behavior, and schema validation |
| `tools/` | Schema validation script (`schema-check`), bootstrap, and check commands |

### Phase 1 API contract

| Method | Path | Purpose | Body | Response |
| --- | --- | --- | --- | --- |
| `POST` | `/api/session` | Create a learning session | none | `{ "session_id": "uuid", "state": "IDLE" }` |
| `POST` | `/api/session/{id}/goal` | Submit a learning goal | `LearningGoal` | SSE stream with status and `FeasibilityResult` |
| `POST` | `/api/session/{id}/profile/answer` | Submit a profile answer | `{ "question_id": "...", "answer": "..." }` | SSE stream with next question or completed `UserProfile` |
| `GET` | `/api/session/{id}/curriculum` | Get the curriculum | none | `CurriculumPlan` |
| `GET` | `/api/session/{id}/chapter/{ch_id}` | Start or continue chapter teaching | none | SSE stream with chapter content |
| `POST` | `/api/session/{id}/chapter/{ch_id}/ask` | Ask a question inside a chapter | `{ "question": "..." }` | SSE stream with `Message` |
| `POST` | `/api/session/{id}/chapter/{ch_id}/complete` | Mark a chapter complete | none | `ChapterProgress` |

All JSON endpoints return `application/json`. Streaming endpoints return `text/event-stream`. Error responses use:

```json
{ "error": { "code": "string", "message": "string" } }
```

### State machine

```text
IDLE → GOAL_INPUT → FEASIBILITY_CHECK → PROFILE_COLLECTION → CURRICULUM_PLANNING → CHAPTER_LEARNING → COMPLETED
```

Any state may transition to `ERROR`. `ERROR` may retry the previous state or reset to `IDLE`.

Rules:

- A session has exactly one active state transition at a time.
- Phase 1 may store state in memory or JSON files. Phase 2 moves persistent state to SQLite or PostgreSQL.
- Disconnected clients resume by `session_id`.
- Invalid transitions must return structured errors and must be tested.

### SSE event contract

| Event | Purpose | Data |
| --- | --- | --- |
| `chunk` | Streamed LLM text | `{ "content": "string", "index": number }` |
| `status` | State or step status | `{ "state": "string", "message": "string" }` |
| `error` | Recoverable or fatal error | `{ "code": "string", "message": "string" }` |
| `done` | Step completion | `{ "result": <SchemaType> }` |
| `ping` | Keepalive every 15 seconds | `{}` |

The server should keep a bounded replay buffer for recent events and support `Last-Event-ID` when practical.

## Advanced Learning-Material Workflow

These capabilities are important but must be phased after the MVP.

### Import pipeline

Supported source types by Phase 2.5+:

- PDF files, with OCR only when text extraction fails and with clear uncertainty metadata.
- Plain text and Markdown files.
- Website content through a controlled fetch/extract pipeline that records URL, title, access time, content hash, and usage notes.

Imported materials must become structured source documents with metadata:

```text
source_id, source_type, title, origin, checksum, extracted_at, language, license_or_usage_note, chunks[]
```

LLM-generated learning content that uses imported materials must cite source chunks. The system must surface uncertainty and conflicts instead of hiding them.

### Typst and PDF export pipeline

Learning content should be exported through Typst as the intermediate representation:

```text
structured lesson → Typst document → typst compile → PDF artifact
```

Rules:

- Typst compilation is a tool execution step, not an LLM claim.
- Compile errors are returned as structured diagnostics.
- Generated PDFs and intermediate Typst files are artifacts unless the user explicitly saves or exports them.
- Exports must validate missing assets, broken references, unsupported formulas, and source citation integrity.

## Testing and Quality Gates

### Mandatory gates by area

| Area | Required checks |
| --- | --- |
| Rust | `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test` |
| Web | `npm run lint`, `npm run typecheck`, and the configured test command |
| Schemas | JSON Schema syntax validation and fixture validation |
| Prompts | Contract tests using mock LLM outputs that must validate against schemas |
| API | REST and SSE integration tests with fixed mock responses |
| State machine | Valid transitions, invalid transitions, retries, reset, and resume behavior |
| Sandbox, Phase 2+ | timeout, memory, CPU, disk, disabled network, malicious inputs, non-zero exits |
| Import/export, Phase 2.5+ | PDF/text/web fixtures, extraction diagnostics, Typst compile success and failure |
| Docs | no accidental secrets, no unresolved accepted-phase placeholders, no Chinese text after this rewrite |

Default tests must not call paid LLM APIs, use private user data, or execute untrusted user code on the host.

## Static Analysis Policy

- Treat warnings as failures in CI for Rust and TypeScript where practical.
- Validate schemas before using them for prompt or API contracts.
- Check generated TypeScript/Rust types against schema source when code generation is introduced.
- Add secret scanning before commits and CI merges.
- Keep dependency additions explicit and justified by the current phase.

## Logging and Observability

Use structured logs. Rust services should use `tracing` and support JSON output.

Required fields where applicable:

```text
timestamp, level, target, request_id, session_id, state, event, duration_ms, error_code
```

LLM call logs may include model name, latency, retry count, token counts, validation status, and redacted error summaries. They must not include API keys, raw private documents, full prompts containing sensitive user data, or unredacted user secrets.

Sandbox logs must include request ID, tool category, configured limits, exit code, timeout flag, duration, and truncated stdout/stderr.

Import/export logs must include source type, checksum, parser status, chunk count, Typst compile status, artifact ID, and diagnostics.

## Security and Privacy Rules

- Never commit API keys, tokens, credentials, private user data, imported private materials, or generated user artifacts.
- UI code must never call LLM providers directly.
- Core must validate LLM structured output before using it.
- User-submitted code must never run on the host. Phase 1 does not run user code. Phase 2+ must use a sandbox with resource limits and disabled network by default.
- Plugins must not access files, network, shell commands, databases, secrets, or other plugins directly.
- Logs must be safe to share with developers after redaction.

## Module Map

```text
schemas/          # shared protocol definitions
crates/           # Rust services and libraries
prompts/          # versioned LLM prompt templates
apps/             # user-facing applications
sandboxes/        # isolated execution environments, Phase 2+
plugins/          # permissioned learning extensions, Phase 3+
tests/            # integration, contract, E2E, and security tests
tools/            # validation, bootstrap, import/export, and developer tooling
assets/           # fonts, icons, scene assets, and licensed learning assets
docs-internal/    # ADRs, threat models, research notes, experiments
```

## Do Not

- Do not move advanced features into Phase 1 to make the roadmap look more impressive.
- Do not hide deterministic failures behind LLM-generated explanations.
- Do not duplicate protocol definitions across modules without a canonical schema source.
- Do not add a dependency or service because it is part of the long-term vision unless the current phase needs it.
- Do not create marketing copy in agent instruction files.
