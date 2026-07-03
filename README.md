# Blup

Blup is an AI interactive learning-agent platform. A learner enters a goal, the system checks feasibility, collects a learner profile, generates a personalized curriculum, and teaches chapter by chapter.

## Current Status

Phase 0, Phase 1, and Phase 2 are complete in the codebase. **Phase 2.5 is in progress**, with the current focus on desktop-aware materials import, Typst/PDF export hardening, sandbox registry drift checks, and developer gate coverage.

| Phase | Goal | Status |
| --- | --- | --- |
| Phase 0 | Repository foundation, validation, scripts, observability policy | Complete |
| Phase 1 | Web learning assistant: goal feasibility → profile → curriculum → chapter teaching | Complete |
| Phase 2 | Exercises, assessment, sandboxed execution, persistence | Complete |
| Phase 2.5 | Desktop packaging, source import, Typst/PDF export | In progress |
| Phase 3 | Plugin system and Bevy interactive scenes | Planned |

## Architecture Direction

```text
blup/
├── apps/           # Web UI and desktop shell
├── assets/         # fonts, icons, licensed media, scene assets
├── crates/         # Rust agent core, storage, assessment, sandbox, content pipeline
├── docs-internal/  # ADRs, threat models, research notes, experiments
├── plugins/        # Phase 3 domain plugins
├── prompts/        # versioned LLM prompt templates
├── sandboxes/      # isolated execution environments
├── schemas/        # shared structured protocol definitions
├── tests/          # integration, contract, E2E, and security tests
└── tools/          # developer tooling and validation scripts
```

## Core Principles

- LLMs explain, plan, and tutor; they do not fake deterministic execution.
- Structured schemas define contracts between modules.
- User code execution, math calculation, document compilation, and imports must use real tools with validation and logs.
- Tauri, imports, export, and Docker sandbox execution are Phase 2+ capabilities and must stay behind backend/tool boundaries.

See [`AGENTS.md`](./AGENTS.md) for the canonical implementation plan and agent instructions.

## Development Gates

`./scripts/check` is the default local gate. It now covers Rust format/lint/tests, schema validation, generated sandbox registry drift, generated schema TypeScript drift, Web typecheck/lint/test/build, and Python package tests when tests are enabled.

Useful heavier switches:

```text
./scripts/check --with-e2e      # include mocked Playwright E2E
./scripts/check --with-docker   # include ignored Docker sandbox tests
./scripts/check --with-python   # run Python package tests even with --skip-tests
```

## Materials Interfaces

Phase 2.5 exposes session-aware, safe material reads and website import through the backend:

```text
GET  /api/session/{id}/sources
GET  /api/session/{id}/sources/{source_id}
POST /api/session/{id}/sources/import/website
GET  /api/session/{id}/imports/{job_id}
```

Desktop local file import remains behind Tauri file permissions. Web can list imported materials and trigger guarded website import only.

## License

TBD
