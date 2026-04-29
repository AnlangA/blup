# Blup

Blup is an AI interactive learning-agent platform. A learner enters a goal, the system checks feasibility, collects a learner profile, generates a personalized curriculum, and teaches chapter by chapter.

## Current Status

The project is in planning and early scaffold stage. The immediate target is **Phase 1: a single-user web learning assistant MVP**.

| Phase | Goal | Status |
| --- | --- | --- |
| Phase 0 | Repository foundation, validation, scripts, observability policy | Complete |
| Phase 1 | Web learning assistant: goal feasibility → profile → curriculum → chapter teaching | Complete |
| Phase 2 | Exercises, assessment, sandboxed execution, persistence | Planned |
| Phase 2.5 | Desktop packaging, source import, Typst/PDF export | Planned |
| Phase 3 | Plugin system and Bevy interactive scenes | Planned |

## Architecture Direction

```text
blup/
├── apps/           # Web UI, later desktop shell and Bevy viewer
├── assets/         # fonts, icons, licensed media, future scene assets
├── crates/         # Rust agent core and future service crates
├── docs-internal/  # ADRs, threat models, research notes, experiments
├── plugins/        # future domain plugins
├── prompts/        # versioned LLM prompt templates
├── sandboxes/      # future isolated execution environments
├── schemas/        # shared structured protocol definitions
├── tests/          # integration, contract, E2E, and security tests
└── tools/          # developer tooling and validation scripts
```

## Core Principles

- LLMs explain, plan, and tutor; they do not fake deterministic execution.
- Structured schemas define contracts between modules.
- User code execution, math calculation, document compilation, and imports must use real tools with validation and logs.
- Phase 1 intentionally excludes Tauri, Bevy, WASM plugins, Docker sandboxes, and real code execution.

See [`AGENTS.md`](./AGENTS.md) for the canonical implementation plan and agent instructions.

## License

TBD
