# AGENTS.md

## Purpose

`docs-internal/` stores internal engineering records: Architecture Decision Records, threat models, protocol drafts, research notes, and experiments. It is not a user-facing manual and not a replacement for the root plan.

## Scope

Use this directory for decisions and research that materially affect implementation, security, reliability, or product feasibility.

## Recommended Structure

```text
docs-internal/
├── adr/
│   ├── 0001-use-rust.md
│   ├── 0002-llm-gateway-design.md
│   ├── 0003-phase-1-single-crate.md
│   └── template.md
├── threat-models/
│   ├── plugin-isolation.md
│   ├── sandbox-security.md
│   └── template.md
├── experiments/
│   └── README.md
└── research/
    └── README.md
```

## ADR Format

Each ADR should include:

- Title.
- Status: Proposed, Accepted, Deprecated, or Superseded.
- Context.
- Decision.
- Consequences, including trade-offs.
- Alternatives considered.

## Threat Model Format

Threat models should include:

- Assets.
- Trust boundaries.
- Threats, preferably organized with STRIDE where useful.
- Attack vectors.
- Mitigations.
- Residual risks.
- Tests or monitoring that validate the mitigations.

## Research Note Format

Research notes should include:

- Question being answered.
- Sources reviewed.
- Summary of findings.
- Recommendation.
- Unknowns and follow-up work.

## Testing and Quality Gates

- Security-sensitive designs require an ADR or threat model before implementation.
- External claims should include links or citations.
- Experiments must state whether they are retained, archived, or deleted.

## Logging and Observability

Internal docs may define logging fields and redaction policies. They must not include raw production logs, private learner data, or credentials.

## Security and Privacy Rules

- Do not store API keys, tokens, credentials, or private user data.
- Do not paste unredacted private documents or prompts.
- Do not treat unverified external articles as authoritative facts.

## Do Not

- Do not write marketing copy here.
- Do not create design documents that cannot guide implementation or validation.
- Do not leave major architectural choices undocumented.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
