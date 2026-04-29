use agent_core::models::types::{CreateSessionResponse, ErrorResponse};
use agent_core::state::machine::StateMachine;
use agent_core::state::types::{SessionState, Transition};
use agent_core::validation::schema_validator::SchemaValidator;
use serde_json::json;

#[test]
fn test_learning_goal_schema_validation() {
    let validator = SchemaValidator::new("../schemas");

    // Valid goal
    let valid_goal = json!({
        "description": "Learn Python for data analysis with pandas and numpy",
        "domain": "programming"
    });
    assert!(validator.validate(&valid_goal, "learning_goal").is_ok());

    // Valid goal with optional fields
    let valid_goal_with_context = json!({
        "description": "Learn Rust for systems programming",
        "domain": "programming",
        "context": "I want to build high-performance applications",
        "current_level": "intermediate"
    });
    assert!(validator
        .validate(&valid_goal_with_context, "learning_goal")
        .is_ok());

    // Invalid goal - missing required fields
    let invalid_goal = json!({
        "description": "Learn Python"
    });
    assert!(validator.validate(&invalid_goal, "learning_goal").is_err());

    // Invalid goal - description too short
    let short_desc_goal = json!({
        "description": "Learn",
        "domain": "programming"
    });
    assert!(validator
        .validate(&short_desc_goal, "learning_goal")
        .is_err());
}

#[test]
fn test_feasibility_result_schema_validation() {
    let validator = SchemaValidator::new("../schemas");

    // Valid feasible result
    let feasible_result = json!({
        "feasible": true,
        "reason": "This is a well-defined learning goal",
        "suggestions": ["Start with basics", "Practice daily"],
        "estimated_duration": "3 months"
    });
    assert!(validator
        .validate(&feasible_result, "feasibility_result")
        .is_ok());

    // Valid infeasible result
    let infeasible_result = json!({
        "feasible": false,
        "reason": "This goal is too vague",
        "suggestions": ["Be more specific", "Break it down"]
    });
    assert!(validator
        .validate(&infeasible_result, "feasibility_result")
        .is_ok());

    // Invalid result - missing reason
    let invalid_result = json!({
        "feasible": true
    });
    assert!(validator
        .validate(&invalid_result, "feasibility_result")
        .is_err());
}

#[test]
fn test_user_profile_schema_validation() {
    let validator = SchemaValidator::new("../schemas");

    // Valid profile
    let valid_profile = json!({
        "experience_level": {
            "domain_knowledge": "intermediate"
        },
        "learning_style": {
            "preferred_format": ["text", "exercise-based"]
        },
        "available_time": {
            "hours_per_week": 10
        }
    });
    assert!(validator.validate(&valid_profile, "user_profile").is_ok());

    // Invalid profile - missing required fields
    let invalid_profile = json!({
        "experience_level": {
            "domain_knowledge": "beginner"
        }
    });
    assert!(validator
        .validate(&invalid_profile, "user_profile")
        .is_err());
}

#[test]
fn test_curriculum_plan_schema_validation() {
    let validator = SchemaValidator::new("../schemas");

    // Valid curriculum
    let valid_curriculum = json!({
        "title": "Python Programming Fundamentals",
        "description": "A comprehensive introduction to Python programming",
        "chapters": [
            {
                "id": "ch1",
                "title": "Introduction to Python",
                "order": 1,
                "objectives": ["Understand Python syntax", "Write basic programs"],
                "estimated_minutes": 120
            },
            {
                "id": "ch2",
                "title": "Variables and Data Types",
                "order": 2,
                "objectives": ["Declare variables", "Use different data types"],
                "estimated_minutes": 180
            }
        ],
        "estimated_duration": "5 hours"
    });
    assert!(validator
        .validate(&valid_curriculum, "curriculum_plan")
        .is_ok());

    // Invalid curriculum - empty chapters
    let invalid_curriculum = json!({
        "title": "Empty Course",
        "description": "A course with no chapters",
        "chapters": [],
        "estimated_duration": "0 hours"
    });
    assert!(validator
        .validate(&invalid_curriculum, "curriculum_plan")
        .is_err());
}

#[test]
fn test_chapter_schema_validation() {
    let validator = SchemaValidator::new("../schemas");

    // Valid chapter
    let valid_chapter = json!({
        "id": "ch1",
        "title": "Introduction to Python",
        "order": 1,
        "objectives": ["Understand Python syntax", "Write basic programs"],
        "content": "# Introduction\n\nPython is a versatile programming language...",
        "estimated_minutes": 120
    });
    assert!(validator.validate(&valid_chapter, "chapter").is_ok());

    // Valid chapter without content (content is optional)
    let minimal_chapter = json!({
        "id": "ch2",
        "title": "Variables and Data Types",
        "order": 2,
        "objectives": ["Declare variables", "Use different data types"]
    });
    assert!(validator.validate(&minimal_chapter, "chapter").is_ok());

    // Invalid chapter - missing required fields
    let invalid_chapter = json!({
        "id": "ch3",
        "title": "Incomplete Chapter"
    });
    assert!(validator.validate(&invalid_chapter, "chapter").is_err());
}

#[test]
fn test_message_schema_validation() {
    let validator = SchemaValidator::new("../schemas");

    // Valid assistant message
    let valid_message = json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "role": "assistant",
        "content": "Hello! I'm here to help you learn Python.",
        "timestamp": "2024-01-15T10:30:00Z"
    });
    assert!(validator.validate(&valid_message, "message").is_ok());

    // Valid user message
    let user_message = json!({
        "id": "550e8400-e29b-41d4-a716-446655440001",
        "role": "user",
        "content": "What is a variable?",
        "timestamp": "2024-01-15T10:31:00Z"
    });
    assert!(validator.validate(&user_message, "message").is_ok());

    // Valid system message (system is allowed)
    let system_message = json!({
        "id": "550e8400-e29b-41d4-a716-446655440002",
        "role": "system",
        "content": "System message",
        "timestamp": "2024-01-15T10:32:00Z"
    });
    assert!(validator.validate(&system_message, "message").is_ok());

    // Invalid message - missing required fields
    let invalid_message = json!({
        "id": "550e8400-e29b-41d4-a716-446655440003",
        "role": "assistant",
        "content": "Missing timestamp"
    });
    assert!(validator.validate(&invalid_message, "message").is_err());
}

#[test]
fn test_chapter_progress_schema_validation() {
    let validator = SchemaValidator::new("../schemas");

    // Valid progress
    let valid_progress = json!({
        "chapter_id": "ch1",
        "status": "completed",
        "completion": 100.0,
        "last_accessed": "2024-01-15T10:30:00Z",
        "time_spent": "45 minutes"
    });
    assert!(validator
        .validate(&valid_progress, "chapter_progress")
        .is_ok());

    // Valid in-progress
    let in_progress = json!({
        "chapter_id": "ch2",
        "status": "in_progress",
        "completion": 50.0,
        "last_accessed": "2024-01-15T11:00:00Z"
    });
    assert!(validator.validate(&in_progress, "chapter_progress").is_ok());

    // Invalid progress - missing required fields
    let invalid_progress = json!({
        "chapter_id": "ch1",
        "completion": 100.0
    });
    assert!(validator
        .validate(&invalid_progress, "chapter_progress")
        .is_err());
}

#[test]
fn test_state_machine_full_flow_with_validation() {
    let mut sm = StateMachine::new();
    let validator = SchemaValidator::new("../schemas");

    // Initial state
    assert_eq!(sm.current_state(), SessionState::Idle);

    // Submit goal
    sm.transition(Transition::SubmitGoal).unwrap();
    assert_eq!(sm.current_state(), SessionState::GoalInput);

    // Validate goal
    let goal = json!({
        "description": "Learn Python for data science",
        "domain": "programming"
    });
    assert!(validator.validate(&goal, "learning_goal").is_ok());

    // Submit goal again (triggers feasibility check)
    sm.transition(Transition::SubmitGoal).unwrap();
    assert_eq!(sm.current_state(), SessionState::FeasibilityCheck);

    // Goal is feasible
    sm.transition(Transition::GoalFeasible).unwrap();
    assert_eq!(sm.current_state(), SessionState::ProfileCollection);

    // Profile complete
    sm.transition(Transition::ProfileComplete).unwrap();
    assert_eq!(sm.current_state(), SessionState::CurriculumPlanning);

    // Curriculum ready
    sm.transition(Transition::CurriculumReady).unwrap();
    assert_eq!(sm.current_state(), SessionState::ChapterLearning);

    // Complete chapters
    sm.transition(Transition::ChapterComplete).unwrap();
    assert_eq!(sm.current_state(), SessionState::ChapterLearning);

    sm.transition(Transition::ChapterComplete).unwrap();
    assert_eq!(sm.current_state(), SessionState::ChapterLearning);

    // All chapters done
    sm.transition(Transition::AllChaptersDone).unwrap();
    assert_eq!(sm.current_state(), SessionState::Completed);

    // Reset
    sm.transition(Transition::Reset).unwrap();
    assert_eq!(sm.current_state(), SessionState::Idle);
}

#[test]
fn test_state_machine_error_recovery() {
    let mut sm = StateMachine::new();

    // Start flow
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::GoalFeasible).unwrap();

    // Error occurs
    sm.transition(Transition::ErrorOccurred).unwrap();
    assert_eq!(sm.current_state(), SessionState::Error);

    // Retry
    sm.transition(Transition::Retry).unwrap();
    assert_eq!(sm.current_state(), SessionState::ProfileCollection);

    // Continue flow
    sm.transition(Transition::ProfileComplete).unwrap();
    assert_eq!(sm.current_state(), SessionState::CurriculumPlanning);
}

#[test]
fn test_state_machine_invalid_transitions() {
    let mut sm = StateMachine::new();

    // Cannot skip states
    assert!(sm.transition(Transition::GoalFeasible).is_err());
    assert!(sm.transition(Transition::ProfileComplete).is_err());
    assert!(sm.transition(Transition::CurriculumReady).is_err());
    assert!(sm.transition(Transition::AllChaptersDone).is_err());

    // Cannot go backwards
    sm.transition(Transition::SubmitGoal).unwrap();
    assert!(sm.transition(Transition::Reset).is_err());
}

#[test]
fn test_create_session_response_serialization() {
    let response = CreateSessionResponse {
        session_id: "test-session-123".to_string(),
        state: "IDLE".to_string(),
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["session_id"], "test-session-123");
    assert_eq!(json["state"], "IDLE");

    // Deserialize back
    let deserialized: CreateSessionResponse = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.session_id, "test-session-123");
    assert_eq!(deserialized.state, "IDLE");
}

#[test]
fn test_error_response_serialization() {
    let error = ErrorResponse {
        error: agent_core::models::types::ErrorDetail {
            code: "VALIDATION_ERROR".to_string(),
            message: "Invalid input".to_string(),
        },
    };

    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
    assert_eq!(json["error"]["message"], "Invalid input");
}
