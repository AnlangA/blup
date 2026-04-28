# Schema Module — Implementation Plan

## Module Overview

`schemas/` is the canonical contract layer for all structured data exchanged between Core, UI, prompts, sandboxes, plugins, importers, exporters, and renderers. Every cross-module data type is defined here as a versioned JSON Schema, making this directory the source of truth for the entire platform.

## Phase Scope

| Phase | Deliverables | Status |
|-------|-------------|--------|
| Phase 1 | 7 JSON Schemas for learning flow: `LearningGoal`, `FeasibilityResult`, `UserProfile`, `CurriculumPlan`, `Chapter`, `Message`, `ChapterProgress` | Planned |
| Phase 2 | `AssessmentSpec`, `Exercise`, `EvaluationResult`, `SandboxRequest`, tool result schemas | Planned |
| Phase 2.5 | `SourceDocument`, `SourceChunk`, `ImportJob`, `ExportJob`, `DocumentArtifact` | Planned |
| Phase 3 | `PluginManifest`, `PluginRequest`, `PluginResponse`, `SceneSpec`, `RenderCommand` | Planned |

## Phase 1 Detailed Plan

### File Structure

```
schemas/
├── AGENTS.md
├── plan_phase1.md
├── learning_goal.v1.schema.json
├── feasibility_result.v1.schema.json
├── user_profile.v1.schema.json
├── curriculum_plan.v1.schema.json
├── chapter.v1.schema.json
├── message.v1.schema.json
├── chapter_progress.v1.schema.json
└── fixtures/
    ├── learning_goal/
    │   ├── valid-goal.json
    │   ├── valid-goal-with-context.json
    │   └── invalid-empty-description.json
    ├── feasibility_result/
    │   ├── valid-feasible.json
    │   ├── valid-infeasible.json
    │   └── invalid-missing-reason.json
    ├── user_profile/
    │   ├── valid-complete.json
    │   └── invalid-missing-fields.json
    ├── curriculum_plan/
    │   ├── valid-plan.json
    │   └── invalid-empty-chapters.json
    ├── chapter/
    │   ├── valid-chapter.json
    │   └── invalid-no-objectives.json
    ├── message/
    │   ├── valid-user-message.json
    │   ├── valid-assistant-message.json
    │   └── invalid-unknown-role.json
    └── chapter_progress/
        ├── valid-in-progress.json
        ├── valid-completed.json
        └── invalid-negative-completion.json
```

### Schema Specifications

#### 1. LearningGoal (`learning_goal.v1.schema.json`)

Captures the learner's goal, domain, and optional context.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://blup.dev/schemas/learning_goal.v1.schema.json",
  "title": "LearningGoal",
  "type": "object",
  "version": "1.0.0",
  "required": ["description", "domain"],
  "properties": {
    "description": {
      "type": "string",
      "minLength": 10,
      "maxLength": 2000,
      "description": "What the learner wants to learn"
    },
    "domain": {
      "type": "string",
      "minLength": 2,
      "maxLength": 200,
      "description": "Subject domain (e.g. programming, mathematics, physics)"
    },
    "context": {
      "type": "string",
      "maxLength": 2000,
      "description": "Optional background about why they want to learn this"
    },
    "current_level": {
      "type": "string",
      "enum": ["beginner", "intermediate", "advanced", "unknown"],
      "description": "Self-assessed current level"
    }
  }
}
```

**Key design decisions:**
- `domain` is a free-text field in Phase 1 rather than a closed enum, to avoid prematurely restricting domains. Phase 2 may add a controlled vocabulary.
- `current_level` defaults to `"unknown"` when the learner skips self-assessment.

#### 2. FeasibilityResult (`feasibility_result.v1.schema.json`)

Captures the LLM's feasibility assessment of the learning goal.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://blup.dev/schemas/feasibility_result.v1.schema.json",
  "title": "FeasibilityResult",
  "type": "object",
  "version": "1.0.0",
  "required": ["feasible", "reason"],
  "properties": {
    "feasible": {
      "type": "boolean",
      "description": "Whether the goal is feasible to teach"
    },
    "reason": {
      "type": "string",
      "maxLength": 1000,
      "description": "Explanation of the feasibility assessment"
    },
    "suggestions": {
      "type": "array",
      "items": { "type": "string", "maxLength": 500 },
      "maxItems": 5,
      "description": "Suggestions for adjusting an infeasible goal"
    },
    "estimated_duration": {
      "type": "string",
      "maxLength": 200,
      "description": "Estimated learning duration (e.g. '4-6 weeks')"
    },
    "prerequisites": {
      "type": "array",
      "items": { "type": "string", "maxLength": 300 },
      "maxItems": 10,
      "description": "Recommended prerequisites"
    }
  }
}
```

#### 3. UserProfile (`user_profile.v1.schema.json`)

Captures the learner's background, preferences, and constraints collected through 3-5 rounds of Q&A.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://blup.dev/schemas/user_profile.v1.schema.json",
  "title": "UserProfile",
  "type": "object",
  "version": "1.0.0",
  "required": ["experience_level", "learning_style", "available_time"],
  "properties": {
    "experience_level": {
      "type": "object",
      "required": ["domain_knowledge"],
      "properties": {
        "domain_knowledge": {
          "type": "string",
          "enum": ["none", "beginner", "intermediate", "advanced"],
          "description": "Knowledge level in the target domain"
        },
        "related_domains": {
          "type": "array",
          "items": { "type": "string" },
          "maxItems": 10
        },
        "years_of_experience": {
          "type": "number",
          "minimum": 0,
          "maximum": 50
        }
      }
    },
    "learning_style": {
      "type": "object",
      "required": ["preferred_format"],
      "properties": {
        "preferred_format": {
          "type": "array",
          "items": {
            "type": "string",
            "enum": ["text", "visual", "interactive", "audio", "exercise-based", "project-based"]
          },
          "minItems": 1
        },
        "pace_preference": {
          "type": "string",
          "enum": ["slow_thorough", "moderate", "fast_paced", "self_directed"]
        },
        "notes": { "type": "string", "maxLength": 1000 }
      }
    },
    "available_time": {
      "type": "object",
      "required": ["hours_per_week"],
      "properties": {
        "hours_per_week": { "type": "number", "minimum": 0.5, "maximum": 80 },
        "preferred_session_length_minutes": { "type": "number", "minimum": 5, "maximum": 240 },
        "timezone": { "type": "string" }
      }
    },
    "goals": {
      "type": "object",
      "properties": {
        "primary_goal": { "type": "string", "maxLength": 500 },
        "secondary_goals": {
          "type": "array",
          "items": { "type": "string", "maxLength": 300 },
          "maxItems": 5
        },
        "success_criteria": { "type": "string", "maxLength": 500 }
      }
    },
    "preferences": {
      "type": "object",
      "properties": {
        "language": { "type": "string", "maxLength": 50 },
        "difficulty_bias": {
          "type": "string",
          "enum": ["easier", "standard", "challenging"]
        },
        "feedback_frequency": {
          "type": "string",
          "enum": ["immediate", "end_of_section", "end_of_chapter"]
        }
      }
    }
  }
}
```

#### 4. CurriculumPlan (`curriculum_plan.v1.schema.json`)

The personalized learning path generated from the goal and profile.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://blup.dev/schemas/curriculum_plan.v1.schema.json",
  "title": "CurriculumPlan",
  "type": "object",
  "version": "1.0.0",
  "required": ["title", "chapters", "estimated_duration"],
  "properties": {
    "title": {
      "type": "string",
      "minLength": 3,
      "maxLength": 500,
      "description": "Curriculum title"
    },
    "description": {
      "type": "string",
      "maxLength": 2000,
      "description": "Overview of the learning path"
    },
    "chapters": {
      "type": "array",
      "minItems": 1,
      "maxItems": 50,
      "items": { "$ref": "chapter.v1.schema.json#/properties/metadata" }
    },
    "estimated_duration": {
      "type": "string",
      "maxLength": 200,
      "description": "Estimated total learning duration"
    },
    "prerequisites_summary": {
      "type": "array",
      "items": { "type": "string", "maxLength": 300 },
      "maxItems": 10
    },
    "learning_objectives": {
      "type": "array",
      "items": { "type": "string", "maxLength": 500 },
      "minItems": 1,
      "maxItems": 20
    }
  }
}
```

#### 5. Chapter (`chapter.v1.schema.json`)

A single chapter in the curriculum with metadata and Markdown content.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://blup.dev/schemas/chapter.v1.schema.json",
  "title": "Chapter",
  "type": "object",
  "version": "1.0.0",
  "required": ["id", "title", "order", "objectives"],
  "properties": {
    "id": {
      "type": "string",
      "pattern": "^[a-z0-9]+(-[a-z0-9]+)*$",
      "maxLength": 100,
      "description": "URL-safe chapter identifier"
    },
    "title": {
      "type": "string",
      "minLength": 3,
      "maxLength": 300
    },
    "order": {
      "type": "integer",
      "minimum": 1
    },
    "objectives": {
      "type": "array",
      "items": { "type": "string", "maxLength": 500 },
      "minItems": 1,
      "maxItems": 10
    },
    "prerequisites": {
      "type": "array",
      "items": { "type": "string", "maxLength": 200 },
      "maxItems": 10
    },
    "content": {
      "type": "string",
      "maxLength": 100000,
      "description": "Chapter content in Markdown format"
    },
    "estimated_minutes": {
      "type": "integer",
      "minimum": 1,
      "maximum": 480
    },
    "key_concepts": {
      "type": "array",
      "items": { "type": "string", "maxLength": 200 },
      "maxItems": 30
    },
    "exercises": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "question": { "type": "string", "maxLength": 2000 },
          "type": {
            "type": "string",
            "enum": ["multiple_choice", "short_answer", "coding", "reflection"]
          },
          "difficulty": {
            "type": "string",
            "enum": ["easy", "medium", "hard"]
          }
        },
        "required": ["question", "type"]
      },
      "maxItems": 20
    }
  }
}
```

**Note on `exercises`:** Phase 1 includes inline exercises as structured data within chapters. Phase 2 moves exercise definitions to `AssessmentSpec` schemas for independent validation and grading.

#### 6. Message (`message.v1.schema.json`)

A structured conversation message in the learning dialogue.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://blup.dev/schemas/message.v1.schema.json",
  "title": "Message",
  "type": "object",
  "version": "1.0.0",
  "required": ["id", "role", "content", "timestamp"],
  "properties": {
    "id": { "type": "string", "format": "uuid" },
    "role": {
      "type": "string",
      "enum": ["user", "assistant", "system"]
    },
    "content": {
      "type": "string",
      "maxLength": 20000,
      "description": "Message content in Markdown"
    },
    "timestamp": {
      "type": "string",
      "format": "date-time"
    },
    "content_type": {
      "type": "string",
      "enum": ["text", "question", "exercise", "feedback", "explanation", "example", "summary"],
      "description": "Semantic type of the message for UI rendering decisions"
    },
    "references": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "chapter_id": { "type": "string" },
          "concept": { "type": "string" },
          "source_chunk_id": { "type": "string" }
        }
      },
      "maxItems": 10
    },
    "metadata": {
      "type": "object",
      "properties": {
        "tokens_used": { "type": "integer", "minimum": 0 },
        "model": { "type": "string" },
        "generation_duration_ms": { "type": "integer", "minimum": 0 }
      }
    }
  }
}
```

#### 7. ChapterProgress (`chapter_progress.v1.schema.json`)

Per-chapter progress tracking.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://blup.dev/schemas/chapter_progress.v1.schema.json",
  "title": "ChapterProgress",
  "type": "object",
  "version": "1.0.0",
  "required": ["chapter_id", "status", "completion"],
  "properties": {
    "chapter_id": { "type": "string" },
    "status": {
      "type": "string",
      "enum": ["not_started", "in_progress", "completed", "skipped"]
    },
    "completion": {
      "type": "number",
      "minimum": 0,
      "maximum": 100,
      "description": "Completion percentage (0-100)"
    },
    "time_spent_minutes": { "type": "integer", "minimum": 0 },
    "exercises_completed": { "type": "integer", "minimum": 0 },
    "exercises_total": { "type": "integer", "minimum": 0 },
    "last_accessed": { "type": "string", "format": "date-time" },
    "notes": {
      "type": "array",
      "items": { "type": "string", "maxLength": 1000 },
      "maxItems": 20
    },
    "difficulty_rating": {
      "type": "integer",
      "minimum": 1,
      "maximum": 5,
      "description": "Learner's perceived difficulty (1=too easy, 5=too hard)"
    }
  }
}
```

### Validation Infrastructure

#### Schema Validator Tool

Phase 1 requires a schema validation tool in `tools/schema-validator/`. It must:

1. **Validate schema syntax**: Check every `.schema.json` file against the JSON Schema meta-schema (draft 2020-12).
2. **Validate fixtures**: Check valid fixtures pass and invalid fixtures fail against their schema.
3. **Check cross-references**: Verify `$ref` targets exist within the schema directory.
4. **Check naming conventions**: Enforce `{schema_name}.v{major}.schema.json` naming.
5. **Check version fields**: Verify every schema has a `version` field.

**CLI contract:**

```text
schema-validator validate --all          # validate all schemas and fixtures
schema-validator validate --schema <name> # validate a single schema
schema-validator check-naming             # check naming conventions only
```

Exit code 0 on success, non-zero on any failure. Output JSON diagnostics to stdout.

#### CI Integration

```yaml
# In CI pipeline:
schema-check:
  - schema-validator validate --all
  - schema-validator check-naming
```

### Versioning Rules

| Change Type | Version Bump | Example |
|-------------|-------------|---------|
| Add optional property | Minor (1.0 → 1.1) | Add `difficulty_rating` to ChapterProgress |
| Add enum value (non-breaking) | Minor | Add `"skipped"` to ChapterProgress status |
| Add required property | Major (1.0 → 2.0) | Make `key_concepts` required in Chapter |
| Remove or rename property | Major | Rename `content` to `body` in Chapter |
| Change property type | Major | Change `order` from integer to string |
| Narrow constraints (min/max) | Major | Reduce `maxLength` from 20000 to 5000 |

### Schema Evolution Examples

#### Example 1: Adding an Optional Field (Minor Bump)

**Scenario:** Chapter v1.0 needs a `difficulty_rating` field for learner feedback.

**Before (v1.0):**
```json
{
  "required": ["id", "title", "order", "objectives"],
  "properties": {
    "id": { "type": "string" },
    "title": { "type": "string" },
    "order": { "type": "integer" },
    "objectives": { "type": "array", "items": { "type": "string" } },
    "content": { "type": "string" }
  }
}
```

**After (v1.1):** Add `difficulty_rating` as optional. All v1.0 data remains valid.
```json
{
  "version": "1.1.0",
  "required": ["id", "title", "order", "objectives"],
  "properties": {
    "id": { "type": "string" },
    "title": { "type": "string" },
    "order": { "type": "integer" },
    "objectives": { "type": "array", "items": { "type": "string" } },
    "content": { "type": "string" },
    "difficulty_rating": {
      "type": "integer",
      "minimum": 1,
      "maximum": 5,
      "description": "Learner's perceived difficulty (added in v1.1)"
    }
  }
}
```

**Code changes:**
- Rust: Add `difficulty_rating: Option<i32>` to Chapter struct (backward-compatible)
- TypeScript: Add `difficulty_rating?: number` (optional property)
- No prompt changes needed (field is set by learner action, not LLM)
- No database migration needed (nullable column)

#### Example 2: Renaming a Field (Major Bump)

**Scenario:** `ChapterProgress.completion` (0-100 float) is confusing. Rename to `completion_percent`.

**Before (v1.0):**
```json
{
  "properties": {
    "completion": { "type": "number", "minimum": 0, "maximum": 100 }
  }
}
```

**New file: `chapter_progress.v2.schema.json`:**
```json
{
  "version": "2.0.0",
  "properties": {
    "completion_percent": { "type": "number", "minimum": 0, "maximum": 100 }
  }
}
```

**Migration plan:**
1. Create `chapter_progress.v2.schema.json` (new file).
2. Agent-core supports BOTH v1 and v2 during transition period (30 days):
   ```rust
   fn deserialize_chapter_progress(json: &str) -> ChapterProgress {
       // Try v2 first, fall back to v1
       if let Ok(v2) = serde_json::from_str::<ChapterProgressV2>(json) {
           return v2.into();
       }
       let v1 = serde_json::from_str::<ChapterProgressV1>(json).unwrap();
       v1.into_v2() // Maps completion → completion_percent
   }
   ```
3. Run database migration: `UPDATE chapter_progress SET data = migrate_v1_to_v2(data)`.
4. Update Web UI to use `completion_percent`.
5. After 30 days, remove v1 support.
6. Archive `chapter_progress.v1.schema.json` to `schemas/archive/`.

#### Example 3: Adding a Required Field (Major Bump)

**Scenario:** `LearningGoal` v1.0 needs a mandatory `current_level` field.

**Before (v1.0):**
```json
{ "required": ["description", "domain"] }
```

**After (v2.0):**
```json
{ "required": ["description", "domain", "current_level"] }
```

**Problem:** All existing sessions have LearningGoal data without `current_level`. The migration must provide a default.

**Migration:**
```rust
fn migrate_learning_goal_v1_to_v2(v1: LearningGoalV1) -> LearningGoalV2 {
    LearningGoalV2 {
        description: v1.description,
        domain: v1.domain,
        context: v1.context,
        current_level: "unknown".to_string(), // Default for existing data
    }
}
```

**Database migration:**
```sql
UPDATE sessions
SET goal = jsonb_set(
    jsonb_set(goal, '{current_level}', '"unknown"'),
    '{version}', '"2.0.0"'
)
WHERE goal->>'version' = '1.0.0';
```

#### Example 4: Narrowing Constraints (Major Bump)

**Scenario:** `Message.content` maxLength was 20000. Performance issues with very long messages require reducing to 10000.

**Before (v1.0):** `"maxLength": 20000`
**After (v2.0):** `"maxLength": 10000`

**Impact:** Existing messages longer than 10000 chars will fail v2 validation. Options:
- **Truncate:** Trim existing long messages to 10000 chars in migration (acceptable for old content).
- **Reject new:** Only enforce on new writes; reads use v1 schema for old data.
- **Chunk:** Split long messages into multiple shorter ones (complex migration).

**Recommended approach:** Truncate old content, enforce v2 on new writes:
```sql
UPDATE messages
SET content = left(content, 10000)
WHERE length(content) > 10000;
```

#### Migration Automation

The `schema-validator` tool supports migration testing:

```bash
# Test that all v1 fixtures remain valid under v1.1 (backward compat)
schema-validator migrate --from learning_goal.v1 --to learning_goal.v1.1 --fixtures

# Generate migration script skeleton
schema-validator migrate --from learning_goal.v1 --to learning_goal.v2 --generate-script

# Validate that v1→v2 migration produces valid v2 output
schema-validator migrate --from learning_goal.v1 --to learning_goal.v2 --validate-migration
```

### Type Generation (Phase 2+)

When type generation is introduced:
- **Rust types**: Generate via `schemars` derive macros or a build script that reads `.schema.json` files.
- **TypeScript types**: Generate via `json-schema-to-typescript` or equivalent.
- Generated types must be checked into a `generated/` subdirectory or verified in CI.

### Testing Strategy

| Test Category | Tool | Scope |
|---------------|------|-------|
| Schema syntax | `schema-validator` | Every `.schema.json` file |
| Valid fixtures | `schema-validator` | Each schema has ≥2 valid fixtures |
| Invalid fixtures | `schema-validator` | Each schema has ≥2 invalid fixtures |
| Cross-module consistency | Manual review + CI | Schema fields match API contracts and prompt output specs |
| Version compatibility | `schema-validator` | Schema changes pass fixture tests for previous minor version |

### Phase 2 Schema Additions

Phase 2 adds exercise and sandbox schemas:

```
schemas/
├── assessment_spec.v1.schema.json     # Exercise definitions
├── exercise.v1.schema.json            # Individual exercise
├── evaluation_result.v1.schema.json   # Grading results
├── sandbox_request.v1.schema.json     # Code execution request
├── sandbox_result.v1.schema.json      # Code execution result
└── tool_request.v1.schema.json        # Generic tool request envelope
```

### Phase 2.5 Schema Additions

Import/export schemas:

```
schemas/
├── source_document.v1.schema.json     # Imported document with metadata
├── source_chunk.v1.schema.json        # Chunk of a source document
├── import_job.v1.schema.json          # Import job definition
├── export_job.v1.schema.json          # Export job definition
└── document_artifact.v1.schema.json   # Generated document artifact
```

### Phase 3 Schema Additions

Plugin and scene schemas:

```
schemas/
├── plugin_manifest.v1.schema.json     # Plugin metadata, capabilities, permissions
├── plugin_request.v1.schema.json      # Request to a plugin
├── plugin_response.v1.schema.json     # Response from a plugin
├── scene_spec.v1.schema.json          # Scene specification for Bevy
└── render_command.v1.schema.json      # Render command for Bevy viewer
```

### Dependency Graph

```
schemas/  (no dependencies — leaf module)
  ↑
  ├── crates/agent-core  (validates API payloads and LLM outputs)
  ├── prompts/           (output format references)
  ├── apps/web-ui        (TypeScript types generated from schemas)
  └── tests/             (schema validation tests)
```

### Quality Gates

- [ ] All 7 Phase 1 schemas exist, validate, and have fixtures
- [ ] Schema version field present in every schema
- [ ] Naming convention enforced: `{name}.v{major}.schema.json`
- [ ] Valid fixtures pass validation; invalid fixtures fail with specific errors
- [ ] CI fails on schema syntax errors
- [ ] No API keys, paths, or credentials in schema examples or descriptions
- [ ] Cross-references resolvable (no dangling `$ref`)

### Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Schema too rigid for LLM output variability | Broken validation, frequent retries | Use `maxLength` not minLength for LLM-generated text; add `additionalProperties: true` during early iterations |
| Schema version proliferation | CI complexity, confusion | Max 2 active major versions; auto-archive older fixtures |
| Divergence between schema and code types | Runtime validation failures | Generate types from schemas; CI check verifies generated types match source |
| Missing Phase 2/3 fields discovered during Phase 1 | Schema refactor | Add optional fields with minor version bumps; avoid premature optimization |
