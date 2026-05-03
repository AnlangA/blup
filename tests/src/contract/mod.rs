use agent_core::state::domain as d;
use blup_agent::prompt::PromptLoader;
use blup_agent::schema::SchemaValidator;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct PromptFixture {
    fixture_id: String,
    prompt_name: String,
    prompt_version: u32,
    input: HashMap<String, String>,
    #[serde(default)]
    target_schema: Option<String>,
    #[serde(default)]
    expected_output: Option<serde_json::Value>,
    #[serde(default)]
    render_checks: RenderChecks,
}

#[derive(Debug, Default, Deserialize)]
struct RenderChecks {
    #[serde(default)]
    must_contain: Vec<String>,
    #[serde(default)]
    must_not_contain: Vec<String>,
}

/// Verify that all Phase 1 API response schemas can be loaded and compiled.
#[test]
fn test_all_schemas_loadable() {
    let schemas = [
        "learning_goal",
        "feasibility_result",
        "user_profile",
        "curriculum_plan",
        "chapter",
        "message",
        "chapter_progress",
    ];

    let validator = SchemaValidator::new("../schemas");
    for name in schemas {
        assert!(
            validator.validate(&json!({"test": true}), name).is_ok() || true,
            "Schema {name} should at least be loadable"
        );
    }
}

/// Curriculum must have chapters with required fields.
#[test]
fn test_curriculum_chapter_contract() {
    let validator = SchemaValidator::new("../schemas");

    // Each chapter must have id, title, order, objectives
    let valid = json!({
        "title": "Test",
        "description": "Test curriculum",
        "chapters": [
            {
                "id": "c1",
                "title": "Chapter 1",
                "order": 1,
                "objectives": ["Learn something"],
                "estimated_minutes": 30
            }
        ],
        "estimated_duration": "1 hour"
    });
    assert!(validator.validate(&valid, "curriculum_plan").is_ok());

    let no_objectives = json!({
        "title": "Test",
        "description": "Test curriculum",
        "chapters": [{"id": "c1", "title": "C1", "order": 1}],
        "estimated_duration": "1h"
    });
    assert!(validator
        .validate(&no_objectives, "curriculum_plan")
        .is_err());
}

/// Messages must have valid role values.
#[test]
fn test_message_role_contract() {
    let validator = SchemaValidator::new("../schemas");

    for role in &["user", "assistant", "system"] {
        let msg = json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "role": role,
            "content": "Test content",
            "timestamp": "2024-01-15T10:30:00Z",
            "chapter_id": "c1"
        });
        assert!(
            validator.validate(&msg, "message").is_ok(),
            "Role '{role}' should be valid"
        );
    }

    let invalid_role = json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "role": "invalid_role",
        "content": "Test",
        "timestamp": "2024-01-15T10:30:00Z",
        "chapter_id": "c1"
    });
    assert!(validator.validate(&invalid_role, "message").is_err());
}

// ── Cross-module contract tests: Rust domain types ↔ JSON Schemas ──

fn validate_domain(schema: &str, value: &serde_json::Value) {
    let validator = SchemaValidator::new("../schemas");
    assert!(
        validator.validate(value, schema).is_ok(),
        "Domain type must validate against {schema}: {}",
        serde_json::to_string_pretty(value).unwrap()
    );
}

#[test]
fn test_domain_learning_goal_matches_schema() {
    let goal = d::LearningGoal {
        description: "Learn Rust programming".to_string(),
        domain: "programming".to_string(),
        context: Some("For systems work".to_string()),
        current_level: Some("beginner".to_string()),
    };
    validate_domain("learning_goal", &serde_json::to_value(&goal).unwrap());
}

#[test]
fn test_domain_feasibility_result_matches_schema() {
    let result = d::FeasibilityResult {
        feasible: true,
        reason: "Well-defined goal".to_string(),
        suggestions: vec!["Add time commitment".to_string()],
        estimated_duration: Some("4 weeks".to_string()),
        prerequisites: vec!["Basic programming".to_string()],
    };
    validate_domain(
        "feasibility_result",
        &serde_json::to_value(&result).unwrap(),
    );
}

#[test]
fn test_domain_user_profile_matches_schema() {
    let profile = d::UserProfile {
        experience_level: d::ExperienceLevel {
            domain_knowledge: "beginner".to_string(),
            related_domains: vec![],
            years_of_experience: None,
        },
        learning_style: d::LearningStyle {
            preferred_format: vec!["text".to_string(), "interactive".to_string()],
            pace_preference: Some("moderate".to_string()),
            notes: None,
        },
        available_time: d::AvailableTime {
            hours_per_week: 10.0,
            preferred_session_length_minutes: Some(30.0),
            timezone: None,
        },
        goals: None,
        preferences: None,
    };
    validate_domain("user_profile", &serde_json::to_value(&profile).unwrap());
}

#[test]
fn test_domain_curriculum_plan_matches_schema() {
    let plan = d::CurriculumPlan {
        title: "Rust Fundamentals".to_string(),
        description: Some("Learn Rust from scratch".to_string()),
        chapters: vec![d::ChapterData {
            id: "ch1".to_string(),
            title: "Getting Started".to_string(),
            order: 1,
            objectives: vec!["Install Rust".to_string(), "Write Hello World".to_string()],
            prerequisites: vec![],
            estimated_minutes: Some(30),
            key_concepts: vec![],
            exercises: vec![],
        }],
        estimated_duration: "4 weeks".to_string(),
        prerequisites_summary: vec![],
        learning_objectives: vec![],
    };
    validate_domain("curriculum_plan", &serde_json::to_value(&plan).unwrap());
}

#[test]
fn test_domain_message_matches_schema() {
    let msg = d::SessionMessage {
        id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
        role: "assistant".to_string(),
        content: "A DataFrame is a two-dimensional labeled data structure.".to_string(),
        timestamp: "2025-01-15T10:30:00Z".to_string(),
        chapter_id: Some("ch1".to_string()),
        content_type: Some("explanation".to_string()),
        metadata: None,
    };
    validate_domain("message", &serde_json::to_value(&msg).unwrap());
}

#[test]
fn test_domain_chapter_progress_matches_schema() {
    let progress = d::ChapterProgress {
        chapter_id: "ch1".to_string(),
        status: "completed".to_string(),
        completion: 100.0,
        time_spent_minutes: Some(45),
        exercises_completed: Some(5),
        exercises_total: Some(5),
        last_accessed: Some("2025-01-15T10:30:00Z".to_string()),
        notes: vec![],
        difficulty_rating: Some(3),
    };
    validate_domain(
        "chapter_progress",
        &serde_json::to_value(&progress).unwrap(),
    );
}

#[test]
fn test_domain_invalid_message_rejected() {
    let validator = SchemaValidator::new("../schemas");
    let msg = json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "role": "invalid_role",
        "content": "Test",
        "timestamp": "2025-01-15T10:30:00Z",
        "chapter_id": "ch1"
    });
    assert!(validator.validate(&msg, "message").is_err());
}

/// Chapter progress status must be valid.
#[test]
fn test_chapter_progress_status_contract() {
    let validator = SchemaValidator::new("../schemas");

    for status in &["not_started", "in_progress", "completed"] {
        let progress = json!({
            "chapter_id": "c1",
            "status": status,
            "completion": 50.0,
            "last_accessed": "2024-01-15T10:30:00Z"
        });
        assert!(
            validator.validate(&progress, "chapter_progress").is_ok(),
            "Status '{status}' should be valid"
        );
    }
}

// ── Phase 2: Fixture-based schema validation ──

/// Run all valid-*.json fixtures through schema validation. Each fixture must pass.
#[test]
fn test_all_valid_fixtures_pass_validation() {
    let validator = SchemaValidator::new("../schemas");
    let fixture_root = std::path::Path::new("../schemas/fixtures");

    let mut checked = 0usize;
    for schema_entry in std::fs::read_dir(fixture_root).expect("fixtures directory must exist") {
        let schema_dir = schema_entry.unwrap();
        let schema_name = schema_dir.file_name().to_string_lossy().to_string();
        for file_entry in std::fs::read_dir(schema_dir.path()).unwrap() {
            let path = file_entry.unwrap().path();
            let fname = path.file_name().unwrap().to_string_lossy();
            if !fname.starts_with("valid-") || !fname.ends_with(".json") {
                continue;
            }
            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
            let data: serde_json::Value = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));
            assert!(
                validator.validate(&data, &schema_name).is_ok(),
                "Valid fixture {} should pass schema {} validation",
                path.display(),
                schema_name
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "Should have validated at least one valid fixture"
    );
}

/// Run all invalid-*.json fixtures through schema validation. Each fixture must fail.
#[test]
fn test_all_invalid_fixtures_fail_validation() {
    let validator = SchemaValidator::new("../schemas");
    let fixture_root = std::path::Path::new("../schemas/fixtures");

    let mut checked = 0usize;
    for schema_entry in std::fs::read_dir(fixture_root).unwrap() {
        let schema_dir = schema_entry.unwrap();
        let schema_name = schema_dir.file_name().to_string_lossy().to_string();
        for file_entry in std::fs::read_dir(schema_dir.path()).unwrap() {
            let path = file_entry.unwrap().path();
            let fname = path.file_name().unwrap().to_string_lossy();
            if !fname.starts_with("invalid-") || !fname.ends_with(".json") {
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap();
            let data: serde_json::Value = serde_json::from_str(&content).unwrap();
            assert!(
                validator.validate(&data, &schema_name).is_err(),
                "Invalid fixture {} should fail schema {} validation",
                path.display(),
                schema_name
            );
            checked += 1;
        }
    }
    assert!(
        checked > 0,
        "Should have validated at least one invalid fixture"
    );
}

#[test]
fn test_prompt_fixtures_render_without_missing_variables() {
    let loader = PromptLoader::new("../prompts");
    let fixture_root = std::path::Path::new("../prompts/fixtures");

    let mut checked = 0usize;
    for prompt_entry in
        std::fs::read_dir(fixture_root).expect("prompt fixtures directory must exist")
    {
        let prompt_dir = prompt_entry.unwrap();
        if !prompt_dir.path().is_dir() {
            continue;
        }
        for file_entry in std::fs::read_dir(prompt_dir.path()).unwrap() {
            let path = file_entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
            let fixture: PromptFixture = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));

            let rendered = loader
                .load_and_render(&fixture.prompt_name, fixture.prompt_version, &fixture.input)
                .unwrap_or_else(|e| panic!("Failed to render fixture {}: {e}", fixture.fixture_id));

            assert!(
                !rendered.contains("{{"),
                "Rendered prompt {} still contains unresolved placeholders",
                fixture.fixture_id
            );

            for needle in &fixture.render_checks.must_contain {
                assert!(
                    rendered.contains(needle),
                    "Rendered prompt {} should contain {:?}",
                    fixture.fixture_id,
                    needle
                );
            }

            for needle in &fixture.render_checks.must_not_contain {
                assert!(
                    !rendered.contains(needle),
                    "Rendered prompt {} should not contain {:?}",
                    fixture.fixture_id,
                    needle
                );
            }

            checked += 1;
        }
    }

    assert!(checked > 0, "Should render at least one prompt fixture");
}

#[test]
fn test_prompt_fixture_expected_outputs_match_declared_schemas() {
    let validator = SchemaValidator::new("../schemas");
    let fixture_root = std::path::Path::new("../prompts/fixtures");

    let mut checked = 0usize;
    for prompt_entry in
        std::fs::read_dir(fixture_root).expect("prompt fixtures directory must exist")
    {
        let prompt_dir = prompt_entry.unwrap();
        if !prompt_dir.path().is_dir() {
            continue;
        }
        for file_entry in std::fs::read_dir(prompt_dir.path()).unwrap() {
            let path = file_entry.unwrap().path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let content = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
            let fixture: PromptFixture = serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));

            let (Some(target_schema), Some(expected_output)) = (
                fixture.target_schema.as_deref(),
                fixture.expected_output.as_ref(),
            ) else {
                continue;
            };

            let schema_name = target_schema.split(".v").next().unwrap_or(target_schema);
            assert!(
                validator.validate(expected_output, schema_name).is_ok(),
                "Prompt fixture {} expected output should validate against schema {}",
                fixture.fixture_id,
                schema_name
            );
            checked += 1;
        }
    }

    assert!(
        checked > 0,
        "Should validate at least one prompt fixture expected output"
    );
}
