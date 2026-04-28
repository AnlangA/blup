# AGENTS.md

## Purpose

`apps/` contains user-facing application entry points: the Phase 1 Web UI, later the Tauri desktop shell, and later the Bevy viewer or embedded renderer.

## Scope

### Phase 1 deliverables

- `apps/web-ui`: a React or Svelte single-page application.
- Chat window for learner messages and assistant responses.
- Curriculum sidebar with chapter navigation.
- Chapter content area with Markdown, KaTeX, and CodeMirror 6.
- Simple state routing that mirrors the Agent Core session state.

### Not in Phase 1

- `apps/desktop` Tauri application.
- `apps/bevy-viewer` rendering host.
- Direct code execution, direct LLM calls, or direct file-system access.

## Module Responsibilities

- Render learning UI and user interactions.
- Call only the Rust Agent Core API or, later, Tauri commands exposed by the trusted backend.
- Keep long-lived learning state in Core, not scattered across UI components.
- Display streamed SSE events, structured errors, validation failures, and retry actions clearly.

## Implementation Plan by Phase

- Phase 1: Web UI over REST and SSE.
- Phase 2: Assessment UI and progress views for persisted sessions.
- Phase 2.5: Tauri packaging and local import/export permissions.
- Phase 3: Optional Bevy scene display embedded beside the Web UI when a validated `SceneSpec` is available.

## Commands

Use repository-level commands when available:

```text
scripts/dev
scripts/check
```

If `apps/web-ui` defines local package scripts, they must include lint and typecheck commands.

## Testing and Quality Gates

- Component tests for chat, curriculum navigation, markdown rendering, and error states.
- SSE integration tests with mocked streams.
- Accessibility checks for keyboard navigation and readable content.
- Type checks must pass before merging UI changes.

## Logging and Observability

- UI logs must not include API keys, raw imported private content, or unredacted learner data.
- Include `session_id` and `request_id` when reporting client errors to Core.
- Surface backend diagnostic IDs to users instead of raw internal stack traces.

## Security and Privacy Rules

- The UI must never call an LLM provider directly.
- The UI must never run user code, compile code, or execute system commands.
- Local file import in Phase 2.5 must go through approved Tauri/backend permissions.
- Treat imported documents as private user data by default.

## Do Not

- Do not implement Agent business logic in frontend components.
- Do not let Bevy replace traditional text, form, chat, and code-display UI.
- Do not store secrets in browser storage.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../crates/AGENTS.md`](../crates/AGENTS.md)
- [`../schemas/AGENTS.md`](../schemas/AGENTS.md)
