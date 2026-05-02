# AGENTS.md

## Purpose

`tests/` contains integration tests, contract tests, end-to-end tests, sandbox security tests, plugin tests, import/export tests, and learning-flow regression tests.

## Scope

### Phase 1 deliverables

- Core learning-flow integration tests.
- State-machine transition tests.
- Schema validation tests.
- HTTP API tests.
- SSE stream tests.
- Prompt contract tests with mock LLM outputs.

### Future deliverables

- Sandbox security and resource-limit tests in Phase 2. **Completed** (mock executor tests, integration tests for storage and assessment).
- Assessment engine tests in Phase 2. **Completed** (all exercise types, determinism, boundary cases).
- Import/export and Typst compilation tests in Phase 2.5.
- Plugin contract tests in Phase 3.
- Full E2E tests once the UI and backend are stable.

## Module Responsibilities

- Make the learning flow regression-testable.
- Verify schemas, prompts, APIs, state transitions, and tool boundaries.
- Prefer deterministic fixtures and mocks over live external services.
- Test failures, timeouts, invalid inputs, and malicious inputs as first-class cases.

## Coverage Targets

| Phase | Target | Focus |
| --- | --- | --- |
| Phase 1 | 80%+ core-flow coverage | goal feasibility, profile collection, curriculum planning, chapter teaching |
| Phase 2 | 90%+ sandbox path coverage and 85%+ assessment-engine coverage | timeouts, limits, malicious input, scoring |
| Phase 2.5 | import/export fixture coverage | PDF/text/web extraction, Typst compile diagnostics |
| Phase 3 | 80%+ plugin-system coverage | manifests, permissions, lifecycle, contract tests |

Coverage targets guide quality; critical security and state-machine paths must be tested even if line coverage is high.

## Testing and Quality Gates

- No default test may call a paid LLM API.
- No test may use real private learner data.
- No test may run untrusted code on the host.
- Snapshot tests may be used for structured output when snapshots are reviewed.
- Property-based tests should cover boundary values for schemas and state transitions where useful.
- Failing sandbox exits, timeouts, and validation errors must be asserted, not ignored.

## Logging and Observability

Tests should assert structured error codes and diagnostic IDs where relevant. Test logs must remain redacted and should not include secrets or full private fixture content.

## Security and Privacy Rules

- Use synthetic fixtures only.
- Keep malicious input fixtures isolated and clearly labeled.
- Do not require developer machines to have private credentials to run the default test suite.

## Do Not

- Do not make tests depend on live LLM output.
- Do not skip invalid-transition tests.
- Do not treat import/export or sandbox failures as non-critical once those features exist.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../crates/AGENTS.md`](../crates/AGENTS.md)
- [`../schemas/AGENTS.md`](../schemas/AGENTS.md)
- [`../sandboxes/AGENTS.md`](../sandboxes/AGENTS.md)
