# AGENTS.md

## Purpose

`schemas/` is the canonical contract layer for structured data exchanged between Core, UI, prompts, sandboxes, plugins, importers, exporters, and renderers.

## Scope

### Phase 1 deliverables

JSON Schemas for:

- `LearningGoal`
- `FeasibilityResult`
- `UserProfile`
- `CurriculumPlan`
- `Chapter`
- `Message`
- `ChapterProgress`

### Future schemas

- `AssessmentSpec`, `Exercise`, and `EvaluationResult` for Phase 2.
- `SandboxRequest`, `ToolRequest`, and tool result schemas for Phase 2.
- `SourceDocument`, `SourceChunk`, `ImportJob`, `ExportJob`, and `DocumentArtifact` for Phase 2.5.
- `PluginManifest`, `PluginRequest`, `PluginResponse`, `SceneSpec`, and `RenderCommand` for Phase 3.

## Module Responsibilities

- Define the source of truth for cross-module data.
- Keep schemas versioned and backward compatibility explicit.
- Provide fixtures for valid and invalid examples.
- Support generation of Rust and TypeScript types when introduced.

## Versioning Rules

- Each long-lived schema must include a version field.
- Backward-compatible optional additions increment the minor version.
- Breaking changes increment the major version.
- File names should follow:

```text
{schema_name}.v{major}.schema.json
```

## Phase 1 Schema Notes

- `LearningGoal` captures the learner's goal, domain, and optional context.
- `FeasibilityResult` captures feasibility, reason, suggestions, and estimated duration.
- `UserProfile` captures experience, background, available time, learning style, and preferences.
- `CurriculumPlan` captures chapters, prerequisites, objectives, and estimated duration.
- `Chapter` captures chapter metadata and Markdown content.
- `Message` captures structured conversation messages.
- `ChapterProgress` captures per-chapter progress state.

## Testing and Quality Gates

- Validate every schema file for syntax.
- Validate representative valid and invalid fixtures.
- Test compatibility for versioned schema changes.
- Prompt outputs and API payloads must be checked against schemas before state changes.

## Logging and Observability

Schema validation errors should include schema name, schema version, field path, request ID, and a redacted error summary. Do not log full private payloads.

## Security and Privacy Rules

- Do not include API keys, credentials, system paths, or sensitive implementation details in schemas or examples.
- Do not pass raw unvalidated LLM output between modules.
- Do not duplicate protocol definitions across modules without generated or documented synchronization.

## Do Not

- Do not create long-lived unversioned schemas.
- Do not use arbitrary strings where enums or structured objects are required.
- Do not mix future schemas into Phase 1 deliverables without a clear phase label.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../crates/AGENTS.md`](../crates/AGENTS.md)
- [`../prompts/AGENTS.md`](../prompts/AGENTS.md)
- [`../tests/AGENTS.md`](../tests/AGENTS.md)
