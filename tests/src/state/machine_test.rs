use std::collections::HashMap;

use agent_core::state::machine::StateMachine;
use agent_core::state::types::{SessionState, Transition};
use blup_agent::prompt::PromptLoader;
use blup_agent::schema::SchemaValidator;

#[test]
fn test_initial_state_is_idle() {
    let sm = StateMachine::new();
    assert_eq!(sm.current_state(), SessionState::Idle);
}

#[test]
fn test_full_happy_path_transitions() {
    let mut sm = StateMachine::new();

    sm.transition(Transition::SubmitGoal).unwrap();
    assert_eq!(sm.current_state(), SessionState::GoalInput);

    sm.transition(Transition::SubmitGoal).unwrap();
    assert_eq!(sm.current_state(), SessionState::FeasibilityCheck);

    sm.transition(Transition::GoalFeasible).unwrap();
    assert_eq!(sm.current_state(), SessionState::ProfileCollection);

    sm.transition(Transition::ProfileComplete).unwrap();
    assert_eq!(sm.current_state(), SessionState::CurriculumPlanning);

    sm.transition(Transition::CurriculumReady).unwrap();
    assert_eq!(sm.current_state(), SessionState::ChapterLearning);

    sm.transition(Transition::ChapterComplete).unwrap();
    assert_eq!(sm.current_state(), SessionState::ChapterLearning);

    sm.transition(Transition::AllChaptersDone).unwrap();
    assert_eq!(sm.current_state(), SessionState::Completed);

    sm.transition(Transition::Reset).unwrap();
    assert_eq!(sm.current_state(), SessionState::Idle);
}

#[test]
fn test_infeasible_goal_returns_to_goal_input() {
    let mut sm = StateMachine::new();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::GoalInfeasible).unwrap();
    assert_eq!(sm.current_state(), SessionState::GoalInput);
}

#[test]
fn test_invalid_transitions_fail() {
    let mut sm = StateMachine::new();
    assert!(sm.transition(Transition::GoalFeasible).is_err());
    assert!(sm.transition(Transition::ProfileComplete).is_err());
    assert!(sm.transition(Transition::AllChaptersDone).is_err());
    assert!(sm.transition(Transition::ChapterComplete).is_err());
    assert!(sm.transition(Transition::CurriculumReady).is_err());
}

#[test]
fn test_error_and_retry() {
    let mut sm = StateMachine::new();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::GoalFeasible).unwrap();

    sm.transition(Transition::ErrorOccurred).unwrap();
    assert_eq!(sm.current_state(), SessionState::Error);

    sm.transition(Transition::Retry).unwrap();
    assert_eq!(sm.current_state(), SessionState::ProfileCollection);
}

#[test]
fn test_error_reset() {
    let mut sm = StateMachine::new();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::ErrorOccurred).unwrap();
    assert_eq!(sm.current_state(), SessionState::Error);
    sm.transition(Transition::Reset).unwrap();
    assert_eq!(sm.current_state(), SessionState::Idle);
}

#[test]
fn test_error_retry_without_previous_state() {
    let mut sm = StateMachine::new();
    sm.transition(Transition::ErrorOccurred).unwrap();
    let result = sm.transition(Transition::Retry);
    assert!(result.is_ok());
    assert_eq!(sm.current_state(), SessionState::Idle);
}

#[test]
fn test_session_state_display() {
    assert_eq!(SessionState::Idle.to_string(), "IDLE");
    assert_eq!(SessionState::GoalInput.to_string(), "GOAL_INPUT");
    assert_eq!(
        SessionState::ChapterLearning.to_string(),
        "CHAPTER_LEARNING"
    );
    assert_eq!(SessionState::Completed.to_string(), "COMPLETED");
}

#[test]
fn test_prompt_loader_loads_templates() {
    let loader = PromptLoader::new("../prompts");
    let template = loader.load("feasibility_check", 1);
    assert!(template.is_ok());
    assert!(template
        .unwrap()
        .contains("Evaluate whether a learning goal"));
}

#[test]
fn test_prompt_loader_renders_variables() {
    let loader = PromptLoader::new("../prompts");
    let template = loader.load("feasibility_check", 1).unwrap();
    let mut vars = HashMap::new();
    vars.insert("learning_goal".to_string(), "Learn Python".to_string());
    vars.insert("domain".to_string(), "programming".to_string());
    vars.insert("context".to_string(), "No experience".to_string());

    let rendered = loader.render(&template, &vars);
    assert!(rendered.contains("Learn Python"));
    assert!(rendered.contains("programming"));
}

#[test]
fn test_prompt_loader_template_not_found() {
    let loader = PromptLoader::new("../prompts");
    let result = loader.load("nonexistent_template", 1);
    assert!(result.is_err());
}

#[test]
fn test_schema_validator_accepts_valid_data() {
    let validator = SchemaValidator::new("../schemas");
    let valid = serde_json::json!({
        "description": "Learn Python for data analysis with pandas",
        "domain": "programming"
    });
    let result = validator.validate(&valid, "learning_goal");
    assert!(result.is_ok());
}

#[test]
fn test_schema_validator_rejects_missing_required_fields() {
    let validator = SchemaValidator::new("../schemas");
    let invalid = serde_json::json!({
        "description": "Learn Python"
    });
    let result = validator.validate(&invalid, "learning_goal");
    assert!(result.is_err());
}

#[test]
fn test_schema_validator_schema_not_found() {
    let validator = SchemaValidator::new("../schemas");
    let data = serde_json::json!({"test": true});
    let result = validator.validate(&data, "nonexistent_schema");
    assert!(result.is_err());
}
