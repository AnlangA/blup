# AGENTS.md

## Purpose

`prompts/` contains versioned prompt templates used by Agent Core. Prompts are business rules and contracts, not throwaway strings.

## Scope

### Phase 1 deliverables

- `feasibility_check.v1.prompt.md`
- `profile_collection.v1.prompt.md`
- `curriculum_planning.v1.prompt.md`
- `chapter_teaching.v1.prompt.md`
- `question_answering.v1.prompt.md`

Each template must define inputs, expected output, schema reference, safety constraints, and examples where useful.

### Future deliverables

- Assessment-generation prompts.
- Answer-evaluation prompts.
- Import-grounded lesson-generation prompts.
- Plugin interaction prompts.

## Module Responsibilities

- Keep prompts versioned and reviewable.
- Require structured output when downstream code needs structure.
- Explicitly forbid fake tool results, fake citations, fake code execution, and fake calculation.
- Support schema validation and prompt contract tests.

## Prompt Template Format

Each prompt should include:

- Template name and version.
- Purpose.
- Input variables.
- Output format with schema reference.
- Safety and privacy rules.
- Handling for uncertainty and missing information.
- Few-shot examples only when they improve reliability.

File names should follow:

```text
{template_name}.v{major}.prompt.md
```

## Testing and Quality Gates

- Prompt outputs used by Core must validate against schemas.
- Contract tests should use fixed mock model responses.
- Prompt-injection cases should be tested for imported materials and user questions.
- Prompt changes that alter output shape require schema or parser review.

## Logging and Observability

Prompt rendering errors should include template name, version, missing variable names, and request ID. Do not log full prompts when they contain private user data or imported private material.

## Security and Privacy Rules

- Do not hard-code API keys, tokens, credentials, private paths, or private user data.
- Do not instruct the model to invent computation, execution, retrieval, or citations.
- Treat imported documents as untrusted input and protect against prompt injection.
- Make uncertainty explicit when the model lacks verified evidence.

## Do Not

- Do not scatter prompts inside UI components.
- Do not create unversioned prompt templates.
- Do not rely on prompt wording alone for security. Core must enforce validation and permissions.

## Related Files

- [`../AGENTS.md`](../AGENTS.md)
- [`../schemas/AGENTS.md`](../schemas/AGENTS.md)
- [`../crates/AGENTS.md`](../crates/AGENTS.md)
