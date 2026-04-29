# AGENTS.md

## Purpose

`tools/` contains developer utilities, validation tools, code generators, schema checkers, prompt testers, sandbox builders, asset processors, import/export tools, and local operational scripts.

## Scope

### Phase 0 and Phase 1 deliverables

- `schema-check`: Validate all schema files and fixtures in CI (implemented as `scripts/schema-check`).
- `bootstrap`: Verify required development tools (implemented as `scripts/bootstrap`).
- `check`: Run all repository validators (implemented as `scripts/check`).

### Future deliverables

- Prompt tester.
- Sandbox image builder.
- Asset optimizer.
- Plugin builder.
- Schema generator.
- Import pipeline tools.
- Typst export and PDF compilation helpers.

## Module Responsibilities

- Make development workflows repeatable and auditable.
- Declare inputs, outputs, side effects, and security limits for every tool.
- Support CI without requiring private credentials.
- Keep source files and generated artifacts clearly separated.

## Planned Tooling

| Tool | Phase | Purpose |
| --- | --- | --- |
| `schema-check` | Phase 1 | Validate all schema files and fixtures in CI |
| `bootstrap` | Phase 0 | Check Rust, Node, package manager, and phase-specific tools |
| `check` | Phase 0 | Run formatters, linters, tests, and schema checks |
| `prompt-tester` | Phase 2 | Test prompt templates against fixtures and schema contracts |
| `sandbox-builder` | Phase 2 | Build sandbox images reproducibly |
| `typst-export` | Phase 2.5 | Generate Typst documents and compile PDFs through controlled commands |
| `content-importer` | Phase 2.5 | Extract source documents from PDF, text, Markdown, and websites |
| `plugin-builder` | Phase 3 | Package and validate plugins |
| `asset-optimizer` | Phase 3 | Optimize assets with recorded provenance |

## Testing and Quality Gates

- Tools must return non-zero exit codes on failure.
- Tools must have deterministic fixtures where practical.
- Tools that execute external commands must show the command category and capture structured diagnostics.
- Import/export tools must test success and failure cases.

## Logging and Observability

Tool logs should include tool name, version, input summary, output path or artifact ID, duration, and structured errors. Do not log secrets or full private documents.

## Security and Privacy Rules

- Do not delete user files by default.
- Do not upload local data by default.
- Do not hide external command execution.
- Do not hard-code local absolute paths, credentials, or personal configuration.
- Treat imported documents as private unless explicitly marked as public fixtures.

## Do Not

- Do not make tooling carry production business logic.
- Do not generate files into source directories without clear naming and review.
- Do not require paid external services for default checks.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../schemas/AGENTS.md`](../schemas/AGENTS.md)
- [`../tests/AGENTS.md`](../tests/AGENTS.md)
