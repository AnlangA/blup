# Docs-Internal Module — Implementation Plan

## Module Overview

`docs-internal/` stores internal engineering records: Architecture Decision Records (ADRs), threat models, protocol drafts, research notes, and experiments. It is the project's engineering memory — not user-facing documentation, not marketing, and not a replacement for the root plan or AGENTS.md files.

## Phase Scope

This module spans all phases. It grows organically as architectural decisions are made, threats analyzed, and research conducted. It has no code deliverables — its output is clarity and traceability for engineering decisions.

## File Structure

```
docs-internal/
├── AGENTS.md
├── plan_phase0.md
├── architecture.md                    # Canonical architecture reference (exists)
├── adr/
│   ├── template.md
│   ├── 0001-use-rust-for-agent-core.md
│   ├── 0002-llm-api-design.md
│   ├── 0003-phase-1-single-crate.md
│   ├── 0004-web-ui-framework-choice.md
│   ├── 0005-prompt-management-strategy.md
│   └── 0006-schema-versioning-policy.md
├── threat-models/
│   ├── template.md
│   ├── llm-output-validation.md
│   ├── plugin-isolation.md
│   ├── sandbox-security.md
│   └── import-pipeline-privacy.md
├── experiments/
│   ├── README.md
│   ├── llm-structured-output-reliability/
│   │   ├── report.md
│   │   └── data/
│   └── sse-streaming-backpressure/
│       └── report.md
└── research/
    ├── README.md
    ├── openai-vs-anthropic-api-comparison.md
    ├── code-sandboxing-approaches.md
    └── wasm-plugin-isolation-survey.md
```

## Phase 0 Deliverables

Phase 0 establishes the documentation infrastructure and records foundational decisions.

### ADRs to Create

#### 0001: Use Rust for Agent Core

**Status:** Accepted
**Context:** The agent core requires strong type safety, reliable async I/O, low resource usage, and integration with Tauri in later phases. Go, Python, and Node.js were considered.
**Decision:** Use Rust with Axum, Tokio, and Serde.
**Consequences:**
- + Strong type system prevents entire classes of runtime errors.
- + Direct Tauri integration path for Phase 2.5.
- + Low memory footprint enables future embedding.
- - Smaller hiring pool than Python/Node.
- - Slower iteration on pure string-processing tasks compared to Python.
**Alternatives considered:** Python (FastAPI) — faster prototyping but weaker type safety, higher resource usage, no Tauri integration. Node.js (Express) — large ecosystem but weaker typing, less suitable for CPU-bound validation.

#### 0002: LLM API Design

**Status:** Accepted (revised)
**Context:** The system needs to call LLM APIs (OpenAI, Anthropic). Decision: call APIs directly from Rust via `reqwest`, or run a Python sidecar using the official SDKs.
**Decision (revised):** Use a **Python LLM Gateway** service (FastAPI) that wraps the official `openai` and `anthropic` Python packages. The Rust `agent-core` calls this gateway over localhost HTTP. This ensures:
- Full compatibility with official SDKs (prompt caching, extended thinking, streaming).
- Automatic updates when providers change their APIs — SDK handles deprecation.
- Clean separation: Rust owns state machine and validation; Python owns AI provider integration.
**Consequences:**
- + Official SDK support for all provider features (no reverse-engineering API changes).
- + Anthropic prompt caching works natively via `cache_control` breakpoints in the SDK.
- + OpenAI streaming, structured outputs, and function calling handled by the SDK.
- + Provider API changes are absorbed by SDK updates, not manual Rust code changes.
- + Circuit breaker, rate limiting, and cost tracking can use Python libraries (`tenacity`, etc.).
- - Adds Python as a runtime dependency (Python 3.12+ required).
- - Adds a localhost HTTP hop (negligible latency: ~1ms on loopback).
- - Agent-core startup must spawn and health-check the gateway process.
- - Two languages in production, but each has a well-defined responsibility.
**Alternatives considered:** Rust `reqwest` directly calling provider APIs — rejected because maintaining parity with official SDKs (especially Anthropic's rapidly evolving API) is unsustainable. PyO3 embedding — rejected due to complexity and GIL issues in async Rust.

#### 0003: Phase 1 Single Crate

**Status:** Accepted
**Context:** The architecture shows many crates (storage, assessment-engine, llm-gateway, etc.), but Phase 1 only needs core orchestration.
**Decision:** Phase 1 ships a single `agent-core` crate. Split into separate crates starting in Phase 2 when persistence and assessment become real requirements.
**Consequences:**
- + Faster iteration with minimal build complexity.
- + Clear Phase 2 boundary — split when the code tells us to, not before.
- - Risk of coupling that makes Phase 2 split harder.
- - Larger single crate may have slower compile times toward end of Phase 1.
**Alternatives considered:** Pre-split into all crates from day one — over-engineering for Phase 1 scope. Microservices — unnecessary operational complexity.

#### 0004: Web UI Framework Choice

**Status:** Proposed
**Context:** Phase 1 requires a SPA with chat, curriculum sidebar, Markdown rendering, KaTeX, and CodeMirror 6.
**Decision:** TBD — evaluate React 18+ with Vite vs Svelte 5 with Vite.
**Evaluation criteria:**
- TypeScript support quality.
- Markdown/KaTeX/CodeMirror integration maturity.
- SSE (EventSource) handling.
- Bundle size for initial load.
- Team familiarity and hiring.
- Tauri integration path for Phase 2.5.

#### 0005: Prompt Management Strategy

**Status:** Proposed
**Context:** Prompts are versioned files in `prompts/`. How does agent-core load them at runtime?
**Decision:** TBD — evaluate embedded at compile time (`include_str!`) vs filesystem at startup vs hot-reload in development.
**Initial recommendation:** Filesystem-loading with a `PromptLoader` that caches templates in memory. Support a `--prompts-dir` CLI flag for development hot-reload. Embed only for release builds.

#### 0006: Schema Versioning Policy

**Status:** Proposed
**Context:** Schemas must evolve while maintaining backward compatibility.
**Decision:** Semantic versioning in filenames (`name.v1.schema.json`). Backward-compatible additions bump minor (tracked in schema `version` field, not filename). Breaking changes create new major version files.
**Consequences:** Multiple major versions coexist during transitions. Agent-core must support validating against both v1 and v2 during a deprecation window.

### Threat Models to Create

#### LLM Output Validation (`threat-models/llm-output-validation.md`)

**Assets:** Learning content integrity, learner trust, system state machine.
**Trust boundary:** Between LLM (untrusted) and Agent Core (trusted).
**Threats:**
| Threat | STRIDE | Severity | Mitigation |
|--------|--------|----------|------------|
| LLM returns malformed JSON | Tampering | Medium | Schema validation before state changes; retry with error feedback |
| LLM hallucinates fake code execution results | Spoofing | High | Prompt rules forbid it; no execution claim path exists in Phase 1 |
| LLM generates harmful/dangerous content | Elevation | High | Content safety rules in prompts; Phase 2 adds content filter |
| LLM output exceeds size limits | DoS | Low | Truncation with structured error; max token limits on LLM calls |
| Prompt injection through user input | Elevation | High | Input delimiters; schema validation catches malformed control structures |
| LLM reveals system prompt | Info Disclosure | Medium | Prompt rules forbid it; monitoring for prompt leakage patterns |

#### Plugin Isolation (`threat-models/plugin-isolation.md`)

**Phase 3 deliverable.** Initial STRIDE analysis:

| Threat | STRIDE | Severity | Attack Vector | Mitigation |
|--------|--------|----------|---------------|------------|
| Plugin reads other learner's data | Info Disclosure | Critical | Plugin process reads shared memory or files | No shared filesystem; plugins get session-scoped temp dirs only |
| Plugin executes shell commands | Elevation | Critical | `os.system()`, `subprocess.run()` in plugin code | Seccomp blocks execve; HTTP containers have no shell |
| Plugin exfiltrates data via DNS | Info Disclosure | High | Plugin encodes data in DNS queries to external server | Network disabled by default; egress filtering |
| Plugin exhausts host memory | DoS | High | Plugin allocates > limit | Container/process memory limit enforced by OS |
| Plugin impersonates Core to learner | Spoofing | Medium | Plugin generates misleading content | All plugin output is labeled as plugin-generated in UI |
| Plugin-to-plugin side channel | Info Disclosure | Medium | Plugin A writes to /tmp, Plugin B reads | Each plugin has isolated tmpfs; no shared directories |
| Plugin survives unload (zombie process) | DoS | Low | Plugin ignores SIGTERM | SIGKILL after timeout; process group kill |

**Sandboxing comparison:**
| Approach | Isolation | Startup | Overhead | Cross-Platform | Recommendation |
|----------|-----------|---------|----------|---------------|---------------|
| HTTP microservice | OS process | ~500ms | Low | Yes (any OS) | Phase 3 default |
| stdin/stdout | OS process | ~200ms | Lowest | Yes | Simple plugins |
| WASM (Wasmtime) | WASM sandbox | ~10ms | Low | Yes (wasmtime runtime) | Evaluate for perf-critical plugins |
| WASM Component Model | WASM + WIT interfaces | ~10ms | Low | Limited (preview 2) | Wait for stabilization |

#### Sandbox Security (`threat-models/sandbox-security.md`)

**Phase 2 deliverable.** STRIDE analysis for code execution sandbox:

| Threat | STRIDE | Severity | Attack Vector | Mitigation |
|--------|--------|----------|---------------|------------|
| Container escape via kernel vuln | Elevation | Critical | Exploit unpatched kernel CVE | Seccomp profile, `--security-opt no-new-privileges`, no CAP_SYS_ADMIN |
| Container escape via Docker socket | Elevation | Critical | Mount `/var/run/docker.sock` | Explicitly blocked: CI test verifies no socket mount |
| Host filesystem access via bind mount | Info Disclosure | Critical | Mount host dir into container | Only tmpfs mounts; CI test verifies no bind mounts |
| Resource exhaustion (fork bomb) | DoS | High | `while True: os.fork()` | `--pids-limit=10` |
| Memory exhaustion | DoS | High | Allocate 100GB array | `--memory=512m`, OOM killer |
| Disk exhaustion | DoS | High | Write 100GB file | `--storage-opt size=100M` |
| Network abuse (outbound) | Repudiation | High | Use sandbox for DDoS | `--network=none` by default |
| Code smuggling via stdout | Info Disclosure | Medium | Encode data in stdout | Output truncated to 64KB; not logged in full |
| CPU exhaustion (crypto mining) | DoS | Medium | Infinite CPU-bound loop | `--cpus=1` + seccomp blocks crypto syscalls |
| Side-channel attack (Spectre/Meltdown) | Info Disclosure | Low | CPU speculation attack | Not practically mitigable in Docker; accept residual risk for Phase 2 |

**Malicious input taxonomy:**
| Category | Example | Expected Behavior |
|----------|---------|-------------------|
| Infinite loops | `while True: pass` | Killed by run timeout |
| Fork bombs | `os.fork()` in loop | Capped by pids-limit |
| Memory bombs | `bytearray(2**30)` | Killed by OOM |
| Disk fillers | `open('/tmp/x','wb').write(b'0'*10**9)` | Blocked by disk limit |
| /dev/null flood | Infinite writes to /dev/null | Killed by timeout |
| /proc enumeration | `os.listdir('/proc')` | Limited by seccomp, read-only rootfs |
| Import of dangerous modules | `import ctypes; ctypes.CDLL(...)` | Seccomp blocks dangerous syscalls |
| Shell injection in code | ``__import__('os').system('rm -rf /')`` | No shell available; seccomp blocks exec |

#### Import Pipeline Privacy (`threat-models/import-pipeline-privacy.md`)

**Phase 2.5 deliverable.** STRIDE analysis:

| Threat | STRIDE | Severity | Mitigation |
|--------|--------|----------|------------|
| Imported PDF contains malware macro | Tampering | High | PDF parsed with pdf-extract (not executed); no macro execution |
| Website fetch follows redirect to internal IP | Info Disclosure | Critical | URL validation before fetch; redirect target validated after fetch; reject private IPs |
| Imported document contains embedded tracking pixels | Info Disclosure | Medium | Images stripped during extraction; only alt-text preserved |
| Learner imports confidential document, chunks become searchable | Info Disclosure | High | Imported documents stored with session-scoped access; not shared between sessions |
| Source document citation leaks private file paths | Info Disclosure | Medium | File paths redacted in citations; only title + public origin used |
| Imported content used for prompt injection | Elevation | High | Imported text placed inside `<imported_source>` delimiters in prompts |

### Research Notes

#### OpenAI vs Anthropic API Comparison (`research/openai-vs-anthropic-api-comparison.md`)

**Question:** What are the key differences between OpenAI and Anthropic APIs that affect our Python LLM Gateway implementation?

**Findings (April 2025):**
- **Transport:** Both use HTTPS REST + SSE streaming. Compatible at the HTTP layer.
- **Authentication:** OpenAI uses `Authorization: Bearer <key>`. Anthropic uses `x-api-key: <key>`. Both supported by the Python SDKs.
- **System messages:** OpenAI includes `system` as a message role. Anthropic's Messages API has `system` as a top-level parameter. The Python gateway abstracts this difference.
- **Structured output:** OpenAI supports `response_format: { type: "json_object" }` for JSON mode with guaranteed valid JSON. Anthropic relies on prompting for JSON output (no native JSON mode). The gateway uses JSON mode when available.
- **Streaming:** OpenAI streams `ChatCompletionChunk` events with `delta.content`. Anthropic streams `text_delta` events. Both Python SDKs provide async iterators.
- **Prompt caching:** Anthropic supports `cache_control: { type: "ephemeral" }` on content blocks for 90% cost reduction on cached prompts. OpenAI has automatic caching on recent requests. The gateway injects Anthropic cache breakpoints on system + long-context messages.
- **Extended thinking:** Anthropic supports `thinking: { type: "enabled", budget_tokens: N }` for chain-of-thought reasoning. The thinking content is returned in separate blocks. The gateway can enable this for complex prompts.
- **Token counting:** OpenAI returns `usage.prompt_tokens` + `completion_tokens`. Anthropic returns `usage.input_tokens` + `output_tokens`. The gateway normalizes to a unified format.
- **Rate limits:** Both return HTTP 429 with `Retry-After` header. The Python `tenacity` library handles backoff for both.

**Recommendation:** The Python LLM Gateway design (ADR 0002) is confirmed. The official `openai` and `anthropic` packages handle all provider-specific differences. The gateway normalizes to a unified `GatewayResponse` format with explicit `provider` field for logging.

#### Code Sandboxing Approaches (`research/code-sandboxing-approaches.md`)

**Updated findings:**
| Approach | Isolation Strength | Cold Start | Memory | Security Track Record | Phase 2 Recommendation |
|----------|-------------------|------------|--------|----------------------|----------------------|
| Docker + seccomp | Moderate (kernel shared) | ~1s | ~20MB + image | Good (CVEs patched quickly) | **Primary** — proven, well-understood |
| gVisor | Strong (user-space kernel) | ~2s | ~30MB + image | Excellent (Google) | Evaluate for multi-tenant scenarios |
| Firecracker | Very strong (KVM microVM) | ~125ms | ~5MB + kernel | Excellent (AWS Lambda) | Phase 3 if multi-tenant needed |
| WASM/Wasmtime | Strong (no kernel access) | ~1ms | ~2MB | Good (Mozilla) | Plugin sandboxing, not code exec |

**Decision:** Docker with seccomp + resource limits for Phase 2 code execution. Re-evaluate gVisor for Phase 3 if multi-tenant or if a critical Docker CVE emerges.

#### WASM Plugin Isolation Survey (`research/wasm-plugin-isolation-survey.md`)

**Updated findings (April 2025):**
- **WASI Preview 2** is stabilized in Wasmtime 20+. Core interfaces: `wasi:cli`, `wasi:http`, `wasi:filesystem`. Component Model still evolving.
- **Component Model** enables typed inter-component communication via WIT (Wasm Interface Types). Tooling (`cargo component`, `wit-bindgen`) is maturing but not yet 1.0.
- **Extism** provides a plugin SDK wrapping wasmtime with a simpler API. Suitable for simple plugins but less flexible than raw wasmtime.
- **Decision:** Start with HTTP microservices for Phase 3 (proven, cross-platform). Monitor WASM Component Model progress. Plan migration path when WIT reaches 1.0 and tooling stabilizes.

### Experiments

Experiments must state:
- **Question being answered.**
- **Methodology.**
- **Results and data** (committed if small, referenced if large).
- **Conclusion:** retained, archived, or deleted.

#### Planned Experiments

1. **LLM Structured Output Reliability** (`experiments/llm-structured-output-reliability/`):
   - **Question:** How reliably do GPT-4o and Claude Sonnet produce valid JSON matching complex schemas?
   - **Method:** Run 100 prompts across 5 schemas with varying complexity. Measure schema validation pass rate, retry success rate, and output diversity.
   - **Status:** Planned for early Phase 1.

2. **SSE Streaming Backpressure** (`experiments/sse-streaming-backpressure/`):
   - **Question:** How does the Axum SSE implementation behave under slow clients and high event rates?
   - **Method:** Simulate slow clients; measure event loss, memory usage, and replay buffer effectiveness.
   - **Status:** Planned for early Phase 1.

## Testing and Quality Gates

- [ ] Every security-sensitive architectural decision has an ADR or threat model.
- [ ] ADRs follow the template: Title, Status, Context, Decision, Consequences, Alternatives.
- [ ] Threat models follow STRIDE where applicable and identify: assets, trust boundaries, threats, mitigations, residual risks, and tests.
- [ ] Research notes cite sources and state recommendation confidence.
- [ ] Experiments are labeled as retained, archived, or deleted.
- [ ] No API keys, tokens, credentials, or private learner data in any doc.
- [ ] No marketing copy — every document must guide implementation or validation decisions.

## Security and Privacy Rules

- Never store API keys, tokens, credentials, or private user data in this directory.
- Never paste unredacted private documents or prompts.
- External claims must include links or citations — do not treat unverified articles as authoritative.
- Threat models must be treated as sensitive: they document attack vectors. Consider whether full threat models should be in a private repository.

## Do Not

- Do not write marketing copy, vision statements, or product roadmaps here (those belong in the repo README or external docs).
- Do not create design documents that cannot guide implementation or validation (no "architecture astronautics").
- Do not leave major architectural choices undocumented — if it affects two or more modules, it needs an ADR.
- Do not archive experiments without stating the conclusion.

## Contributing Guide

### When to Write What

```
I'm making a change that...                    → I should write...

Changes how modules communicate               → ADR (docs-internal/adr/)
Introduces a new attack surface               → Threat Model (docs-internal/threat-models/)
Explores a new technology or approach          → Research Note (docs-internal/research/)
Tests a hypothesis with code                   → Experiment (docs-internal/experiments/)
Changes schemas                                → Schema fixtures (schemas/fixtures/)
Changes LLM behavior                           → Prompt contract test fixtures (prompts/tests/fixtures/)
Adds a feature across modules                  → Integration test (tests/integration/)
Changes public API                             → API test + state machine test
```

### Code Review Checklist

```markdown
## Rust Code Review
- [ ] cargo fmt --check passes
- [ ] cargo clippy --all-targets --all-features -- -D warnings passes
- [ ] cargo test passes
- [ ] No unwrap() in non-test code (use Result or expect with message)
- [ ] No println!() — use tracing::info!/warn!/error!
- [ ] API keys, secrets, or PII never logged or stored directly
- [ ] Error responses follow { "error": { "code": "...", "message": "..." } } format
- [ ] State machine transitions are validated before execution
- [ ] LLM output is schema-validated before use
- [ ] New dependencies justified in Cargo.toml comments

## Python Code Review
- [ ] ruff check passes
- [ ] pytest passes
- [ ] No API keys in code (use environment variables)
- [ ] Async functions use proper httpx timeouts
- [ ] Provider responses validated before returning to Rust

## TypeScript Code Review
- [ ] npm run lint passes
- [ ] npm run typecheck passes
- [ ] npm test passes
- [ ] No direct LLM API calls from UI code
- [ ] No secrets in browser storage (except session_id)
- [ ] SSE connections properly closed on component unmount
- [ ] Error states handled (loading, empty, error, retry)

## Schema Review
- [ ] New schemas follow {name}.v{major}.schema.json naming
- [ ] Version field present
- [ ] Valid and invalid fixtures added to schemas/fixtures/
- [ ] Schema validator passes (./scripts/schema-check)

## Prompt Review
- [ ] Template follows standard format (Purpose, Input, Output, Safety, Examples)
- [ ] Prompt tester passes in mock mode
- [ ] Safety rules partial is included
- [ ] No instruction to fake computation, execution, or citations
- [ ] User input wrapped in delimiters (<user_input> tags)
```

### Creating an ADR

```bash
# Copy the template
cp docs-internal/adr/template.md docs-internal/adr/NNNN-short-title.md

# Edit: Title, Status, Context, Decision, Consequences, Alternatives
# NNNN = next sequential number

# Link from docs-internal/plan_phase0.md ADRs section
```

### Creating a Threat Model

```bash
cp docs-internal/threat-models/template.md docs-internal/threat-models/area-name.md

# Fill in: Assets, Trust Boundaries, Threats (STRIDE), Attack Vectors,
#         Mitigations, Residual Risks, Tests/Monitoring
```

### Creating a Research Note

```markdown
# Research: [Question]

**Date:** YYYY-MM-DD
**Author:** [name]
**Status:** In Progress | Complete

## Question
[One sentence question being answered]

## Sources Reviewed
- [Source 1](url) — Summary of relevance
- [Source 2](url) — Summary of relevance

## Findings
[2-5 paragraphs summarizing what was learned]

## Recommendation
[Clear, actionable recommendation]

## Unknowns
- [Thing we still don't know]
- [Follow-up work needed]
```
