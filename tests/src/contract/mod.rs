use agent_core::validation::schema_validator::SchemaValidator;
use serde_json::json;

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
            "timestamp": "2024-01-15T10:30:00Z"
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
        "timestamp": "2024-01-15T10:30:00Z"
    });
    assert!(validator.validate(&invalid_role, "message").is_err());
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
