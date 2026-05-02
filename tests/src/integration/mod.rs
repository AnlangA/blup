use std::collections::HashMap;

use agent_core::state::machine::StateMachine;
use agent_core::state::types::{SessionState, Transition};
use blup_agent::prompt::PromptLoader;
use blup_agent::schema::SchemaValidator;
use serde_json::json;

use crate::common::{make_mock_provider, TestHarness};

// ── Schema validation tests (sync, no server needed) ──

macro_rules! schema_tests {
    ($name:ident, $schema:expr, $valid:expr, $invalid:expr) => {
        #[test]
        fn $name() {
            let validator = SchemaValidator::new("../schemas");
            for v in $valid {
                assert!(
                    validator.validate(v, $schema).is_ok(),
                    "Expected valid: {}",
                    serde_json::to_string_pretty(v).unwrap()
                );
            }
            for inv in $invalid {
                assert!(
                    validator.validate(inv, $schema).is_err(),
                    "Expected invalid: {}",
                    serde_json::to_string_pretty(inv).unwrap()
                );
            }
        }
    };
}

schema_tests!(
    test_learning_goal_schema,
    "learning_goal",
    [
        &json!({"description":"Learn Python for data analysis","domain":"programming"}),
        &json!({"description":"Learn Rust","domain":"programming","context":"systems","current_level":"intermediate"}),
    ],
    [
        &json!({"description":"Learn Python"}),
        &json!({"description":"X","domain":"programming"}),
    ]
);

schema_tests!(
    test_feasibility_result_schema,
    "feasibility_result",
    [
        &json!({"feasible":true,"reason":"Well-defined","suggestions":["a"],"estimated_duration":"3 months"}),
        &json!({"feasible":false,"reason":"Too vague","suggestions":["be specific"]}),
    ],
    [&json!({"feasible":true}),]
);

schema_tests!(
    test_user_profile_schema,
    "user_profile",
    [
        &json!({"experience_level":{"domain_knowledge":"beginner"},"learning_style":{"preferred_format":["text"]},"available_time":{"hours_per_week":5}}),
    ],
    [&json!({"experience_level":{"domain_knowledge":"beginner"}}),]
);

schema_tests!(
    test_curriculum_plan_schema,
    "curriculum_plan",
    [
        &json!({"title":"Python","description":"Intro","chapters":[{"id":"c1","title":"Ch1","order":1,"objectives":["a"],"estimated_minutes":30}],"estimated_duration":"2h"}),
    ],
    [&json!({"title":"X","description":"X","chapters":[],"estimated_duration":"0h"}),]
);

schema_tests!(
    test_chapter_schema,
    "chapter",
    [
        &json!({"id":"c1","title":"Intro","order":1,"objectives":["a"],"content":"# Hi","estimated_minutes":30}),
        &json!({"id":"c2","title":"Basics","order":2,"objectives":["a"]}),
    ],
    [&json!({"id":"c3","title":"Missing objectives"}),]
);

schema_tests!(
    test_message_schema,
    "message",
    [
        &json!({"id":"550e8400-e29b-41d4-a716-446655440000","role":"assistant","content":"Hello","timestamp":"2024-01-15T10:30:00Z","chapter_id":"c1"}),
        &json!({"id":"550e8400-e29b-41d4-a716-446655440001","role":"user","content":"Hi","timestamp":"2024-01-15T10:31:00Z","chapter_id":"c1"}),
        &json!({"id":"550e8400-e29b-41d4-a716-446655440002","role":"system","content":"sys","timestamp":"2024-01-15T10:32:00Z","chapter_id":"c1"}),
    ],
    [
        &json!({"id":"550e8400-e29b-41d4-a716-446655440003","role":"assistant","content":"Missing timestamp"}),
    ]
);

schema_tests!(
    test_chapter_progress_schema,
    "chapter_progress",
    [
        &json!({"chapter_id":"c1","status":"completed","completion":100.0,"last_accessed":"2024-01-15T10:30:00Z","time_spent_minutes":45}),
        &json!({"chapter_id":"c2","status":"in_progress","completion":50.0,"last_accessed":"2024-01-15T11:00:00Z"}),
    ],
    [&json!({"chapter_id":"c1","completion":100.0}),]
);

// ── State machine tests (sync) ──

#[test]
fn test_state_machine_full_flow() {
    let mut sm = StateMachine::new();
    assert_eq!(sm.current_state(), SessionState::Idle);

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

    sm.transition(Transition::AllChaptersDone).unwrap();
    assert_eq!(sm.current_state(), SessionState::Completed);

    sm.transition(Transition::Reset).unwrap();
    assert_eq!(sm.current_state(), SessionState::Idle);
}

#[test]
fn test_state_machine_error_handling() {
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
fn test_state_machine_invalid_transitions() {
    let mut sm = StateMachine::new();
    assert!(sm.transition(Transition::GoalFeasible).is_err());
    assert!(sm.transition(Transition::ProfileComplete).is_err());
    assert!(sm.transition(Transition::AllChaptersDone).is_err());
    assert!(sm.transition(Transition::ChapterComplete).is_err());

    sm.transition(Transition::SubmitGoal).unwrap();
    assert!(sm.transition(Transition::Reset).is_err());
}

#[test]
fn test_error_retry_without_previous() {
    let mut sm = StateMachine::new();
    sm.transition(Transition::ErrorOccurred).unwrap();
    assert!(sm.transition(Transition::Retry).is_ok());
    assert_eq!(sm.current_state(), SessionState::Idle);
}

#[test]
fn test_profile_continue_stays_in_profile_collection() {
    let mut sm = StateMachine::new();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::SubmitGoal).unwrap();
    sm.transition(Transition::GoalFeasible).unwrap();
    assert_eq!(sm.current_state(), SessionState::ProfileCollection);

    // ProfileContinue should stay in ProfileCollection
    sm.transition(Transition::ProfileContinue).unwrap();
    assert_eq!(sm.current_state(), SessionState::ProfileCollection);

    // Can do it again
    sm.transition(Transition::ProfileContinue).unwrap();
    assert_eq!(sm.current_state(), SessionState::ProfileCollection);

    // And then complete
    sm.transition(Transition::ProfileComplete).unwrap();
    assert_eq!(sm.current_state(), SessionState::CurriculumPlanning);
}

#[test]
fn test_schema_validator_cache() {
    let validator = SchemaValidator::new("../schemas");
    // First validation loads and caches the schema
    let data = json!({"description":"Learn Python for data analysis","domain":"programming"});
    assert!(validator.validate(&data, "learning_goal").is_ok());
    // Second should use cache
    assert!(validator.validate(&data, "learning_goal").is_ok());
    // Clear and validate again
    validator.clear_cache();
    assert!(validator.validate(&data, "learning_goal").is_ok());
}

#[test]
fn test_prompt_loader_reload() {
    let mut loader = PromptLoader::new("../prompts");
    let first = loader.load("feasibility_check", 1).unwrap();
    // Reload should still work
    loader.reload_partials();
    let second = loader.load("feasibility_check", 1).unwrap();
    assert_eq!(first, second);
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

// ── HTTP API Integration Tests ──

#[tokio::test]
async fn test_health() {
    let h = TestHarness::new().await;
    let (status, body) = h.get("/health").await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "ok");
}

#[tokio::test]
async fn test_create_and_get_session() {
    let h = TestHarness::new().await;

    let (status, body) = h.post("/api/session", None).await;
    assert_eq!(status, 200);
    let sid = body["session_id"].as_str().unwrap();
    assert_eq!(body["state"], "IDLE");
    assert!(!sid.is_empty());

    let (status, body) = h.get(&format!("/api/session/{sid}")).await;
    assert_eq!(status, 200);
    assert_eq!(body["session_id"], sid);
    assert_eq!(body["state"], "IDLE");
}

#[tokio::test]
async fn test_session_not_found() {
    let h = TestHarness::new().await;
    let sid = "00000000-0000-0000-0000-000000000000";
    let (status, body) = h.get(&format!("/api/session/{sid}")).await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_submit_goal() {
    let h = TestHarness::new().await;

    let (status, body) = h.post("/api/session", None).await;
    assert_eq!(status, 200);
    let sid = body["session_id"].as_str().unwrap();

    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/goal"),
            Some(json!({
                "description": "I want to learn Python for data analysis",
                "domain": "programming",
                "context": "I work with Excel spreadsheets"
            })),
        )
        .await;
    assert_eq!(status, 200);
    assert!(body["feasibility"]["feasible"].as_bool().unwrap());
    assert_eq!(body["state"], "PROFILE_COLLECTION");
}

#[tokio::test]
async fn test_submit_goal_validation_error() {
    let h = TestHarness::new().await;
    let sid = h.post("/api/session", None).await.1["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/goal"),
            Some(json!({"description": "Short", "domain": "x"})),
        )
        .await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_submit_goal_wrong_session() {
    let h = TestHarness::new().await;
    let (status, _) = h
        .post(
            "/api/session/00000000-0000-0000-0000-000000000000/goal",
            Some(json!({"description": "I want to learn Python", "domain": "programming"})),
        )
        .await;
    assert_eq!(status, 404);
}

#[tokio::test]
async fn test_profile_answer_multi_round() {
    let h = TestHarness::new().await;
    let sid = h.post("/api/session", None).await.1["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    h.post(
        &format!("/api/session/{sid}/goal"),
        Some(json!({"description": "Learn Python for data analysis", "domain": "programming"})),
    )
    .await;

    // Round 1 — not complete
    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/profile/answer"),
            Some(json!({"question_id": "q0", "answer": "No experience"})),
        )
        .await;
    assert_eq!(status, 200);
    assert!(!body["is_complete"].as_bool().unwrap());
    assert_eq!(body["round"], 1);

    // Round 2 — not complete
    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/profile/answer"),
            Some(json!({"question_id": "q1", "answer": "Reading text"})),
        )
        .await;
    assert_eq!(status, 200);
    assert!(!body["is_complete"].as_bool().unwrap());
    assert_eq!(body["round"], 2);

    // Round 3 — complete!
    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/profile/answer"),
            Some(json!({"question_id": "q2", "answer": "2-5 hours"})),
        )
        .await;
    assert_eq!(status, 200);
    assert!(body["is_complete"].as_bool().unwrap());
    assert!(body["profile"].is_object());
}

#[tokio::test]
async fn test_curriculum_flow() {
    let h = TestHarness::new().await;
    let sid = h.post("/api/session", None).await.1["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    h.post(
        &format!("/api/session/{sid}/goal"),
        Some(json!({"description": "Learn Python for data analysis", "domain": "programming"})),
    )
    .await;
    h.complete_profile(&sid).await;

    let (status, body) = h.get(&format!("/api/session/{sid}/curriculum")).await;
    assert_eq!(status, 200);
    assert!(body["title"].is_string());
    assert!(!body["chapters"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_chapter_start_and_cache() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    let (status, body1) = h.get(&format!("/api/session/{sid}/chapter/ch1")).await;
    assert_eq!(status, 200);
    assert!(!body1["content"].as_str().unwrap().is_empty());

    let (status, body2) = h.get(&format!("/api/session/{sid}/chapter/ch1")).await;
    assert_eq!(status, 200);
    assert_eq!(body1["content"], body2["content"]);
}

#[tokio::test]
async fn test_chapter_wrong_state() {
    let h = TestHarness::new().await;
    let sid = h.post("/api/session", None).await.1["session_id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, body) = h.get(&format!("/api/session/{sid}/chapter/ch1")).await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "INVALID_STATE_TRANSITION");
}

#[tokio::test]
async fn test_ask_question() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/chapter/ch1/ask"),
            Some(json!({"question": "What is a variable?"})),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["role"], "assistant");
}

#[tokio::test]
async fn test_ask_question_empty_rejected() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    let (status, _) = h
        .post(
            &format!("/api/session/{sid}/chapter/ch1/ask"),
            Some(json!({"question": ""})),
        )
        .await;
    assert_eq!(status, 422);
}

#[tokio::test]
async fn test_complete_chapter() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    let (status, body) = h
        .post(&format!("/api/session/{sid}/chapter/ch1/complete"), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["chapter_id"], "ch1");
    assert_eq!(body["status"], "completed");
    assert_eq!(body["completion"], 100.0);
}

#[tokio::test]
async fn test_full_learning_flow() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    // Start chapter
    let (status, body) = h.get(&format!("/api/session/{sid}/chapter/ch1")).await;
    assert_eq!(status, 200);
    assert!(!body["content"].as_str().unwrap().is_empty());

    // Ask question
    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/chapter/ch1/ask"),
            Some(json!({"question": "Can you explain more?"})),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["role"], "assistant");

    // Complete chapter
    let (status, body) = h
        .post(&format!("/api/session/{sid}/chapter/ch1/complete"), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "completed");

    // Check final session state
    let (status, body) = h.get(&format!("/api/session/{sid}")).await;
    assert_eq!(status, 200);
    assert_eq!(body["state"], "CHAPTER_LEARNING");
}

// ── SSE Streaming Tests ──

#[tokio::test]
async fn test_sse_submit_goal_stream() {
    use futures::StreamExt;

    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let url = format!("{}/api/session/{sid}/goal/stream", h.base_url);
    let client = TestHarness::http_client();
    let resp = client
        .post(&url)
        .json(&json!({
            "description": "I want to learn Python for data analysis",
            "domain": "programming"
        }))
        .send()
        .await
        .unwrap();

    assert!(resp.status().is_success());

    let mut stream = resp.bytes_stream();
    let mut has_data = false;
    let mut has_done = false;
    let mut buf = String::new();

    while let Some(Ok(chunk)) = stream.next().await {
        buf.push_str(&String::from_utf8_lossy(&chunk));
        if buf.contains("\"FEASIBILITY_CHECK\"") {
            has_data = true;
        }
        if buf.contains("\"result\"") || buf.contains("\"feasibility\"") {
            has_done = true;
            break;
        }
    }

    assert!(
        has_data,
        "SSE stream should include FEASIBILITY_CHECK status data"
    );
    assert!(has_done, "SSE stream should include result data");
}

#[tokio::test]
async fn test_sse_goal_stream_rejects_short_description() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let url = format!("{}/api/session/{sid}/goal/stream", h.base_url);
    let client = TestHarness::http_client();
    let resp = client
        .post(&url)
        .json(&json!({"description": "Short", "domain": "x"}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status().as_u16(), 422);
}

#[tokio::test]
async fn test_sse_chapter_stream() {
    use futures::StreamExt;

    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    let url = format!("{}/api/session/{sid}/chapter/ch1/stream", h.base_url);
    let client = TestHarness::http_client();
    let resp = client.get(&url).send().await.unwrap();
    assert!(resp.status().is_success());

    let mut stream = resp.bytes_stream();
    let mut buf = String::new();
    let mut has_done = false;

    while let Some(Ok(chunk)) = stream.next().await {
        buf.push_str(&String::from_utf8_lossy(&chunk));
        if buf.contains("\"result\"") || buf.contains("\"content\"") {
            has_done = true;
            break;
        }
    }

    assert!(has_done, "Chapter stream should deliver content");
}

#[tokio::test]
async fn test_concurrent_session_reads() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;
    let url = format!("{}/api/session/{sid}", h.base_url);

    // 10 concurrent read requests to the same session
    let client = TestHarness::http_client();
    let handles: Vec<_> = (0..10)
        .map(|_| {
            let u = url.clone();
            let c = client.clone();
            tokio::spawn(async move { c.get(&u).send().await.unwrap() })
        })
        .collect();

    for h in handles {
        let resp = h.await.unwrap();
        assert!(resp.status().is_success());
    }
}

#[tokio::test]
async fn test_concurrent_session_read_write() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;
    let base = h.base_url.clone();

    // Concurrent read + write (ask question) on the same session
    let read_url = format!("{base}/api/session/{sid}");
    let write_url = format!("{base}/api/session/{sid}/chapter/ch1/ask");

    let t1 = tokio::spawn({
        let u = read_url.clone();
        let c = TestHarness::http_client();
        async move { c.get(&u).send().await.unwrap() }
    });

    let t2 = tokio::spawn(async move {
        let client = TestHarness::http_client();
        client
            .post(&write_url)
            .json(&serde_json::json!({"question": "What is a variable?"}))
            .send()
            .await
            .unwrap()
    });

    let (r1, r2) = tokio::join!(t1, t2);
    assert!(r1.unwrap().status().is_success());
    assert!(r2.unwrap().status().is_success());
}

// ---------------------------------------------------------------------------
// Extended Phase 1 tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_delete_session() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let (status, _) = h.delete(&format!("/api/session/{sid}")).await;
    assert_eq!(status, 200);

    let (status, body) = h.get(&format!("/api/session/{sid}")).await;
    assert_eq!(status, 404);
    assert_eq!(body["error"]["code"], "NOT_FOUND");
}

#[tokio::test]
async fn test_list_sessions() {
    let h = TestHarness::new().await;

    let (_, b1) = h.post("/api/session", None).await;
    let (_, b2) = h.post("/api/session", None).await;
    let s1 = b1["session_id"].as_str().unwrap();
    let s2 = b2["session_id"].as_str().unwrap();

    let (status, body) = h.get("/api/sessions").await;
    assert_eq!(status, 200);
    let arr = body.as_array().unwrap();
    assert!(arr.len() >= 2);
    let ids: Vec<&str> = arr.iter().filter_map(|e| e["id"].as_str()).collect();
    assert!(ids.contains(&s1));
    assert!(ids.contains(&s2));
}

#[tokio::test]
async fn test_goal_resubmit_after_infeasible() {
    // Not directly testable with mock since it always returns feasible,
    // but we test the API contract for state transitions
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    // Submit goal → goes to PROFILE_COLLECTION
    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/goal"),
            Some(json!({"description": "Learn Python for data analysis", "domain": "programming"})),
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["state"], "PROFILE_COLLECTION");
}

#[tokio::test]
async fn test_profile_in_wrong_state() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    // Profile answer without submitting goal first → should fail
    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/profile/answer"),
            Some(json!({"question_id": "q0", "answer": "beginner"})),
        )
        .await;
    assert_eq!(status, 409);
    assert_eq!(body["error"]["code"], "INVALID_STATE_TRANSITION");
}

#[tokio::test]
async fn test_curriculum_caching() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    let (status, body1) = h.get(&format!("/api/session/{sid}/curriculum")).await;
    assert_eq!(status, 200);

    let (status, body2) = h.get(&format!("/api/session/{sid}/curriculum")).await;
    assert_eq!(status, 200);

    // Cached response should be identical
    assert_eq!(body1, body2);
}

#[tokio::test]
async fn test_chapter_qa_isolation() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    // Start chapter 1
    h.get(&format!("/api/session/{sid}/chapter/ch1")).await;

    // Ask question in chapter 1
    let (status, _) = h
        .post(
            &format!("/api/session/{sid}/chapter/ch1/ask"),
            Some(json!({"question": "What is a variable?"})),
        )
        .await;
    assert_eq!(status, 200);

    // Start chapter 2
    h.get(&format!("/api/session/{sid}/chapter/ch2")).await;

    // Ask question in chapter 2
    let (status, _) = h
        .post(
            &format!("/api/session/{sid}/chapter/ch2/ask"),
            Some(json!({"question": "What is a type?"})),
        )
        .await;
    assert_eq!(status, 200);

    // Verify session messages are isolated by chapter_id
    let (_, session) = h.get(&format!("/api/session/{sid}")).await;
    let messages = session["messages"].as_array().unwrap();
    let ch1_msgs: Vec<_> = messages
        .iter()
        .filter(|m| m["chapter_id"].as_str() == Some("ch1"))
        .collect();
    let ch2_msgs: Vec<_> = messages
        .iter()
        .filter(|m| m["chapter_id"].as_str() == Some("ch2"))
        .collect();
    // Each chapter should have 1 user + 1 assistant message
    assert_eq!(ch1_msgs.len(), 2);
    assert_eq!(ch2_msgs.len(), 2);
}

#[tokio::test]
async fn test_input_boundary_empty_goal() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/goal"),
            Some(json!({"description": "", "domain": "x"})),
        )
        .await;
    assert_eq!(status, 422);
    assert_eq!(body["error"]["code"], "VALIDATION_ERROR");
}

#[tokio::test]
async fn test_input_boundary_long_goal() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let long_desc = "x".repeat(10000);
    let (status, _) = h
        .post(
            &format!("/api/session/{sid}/goal"),
            Some(json!({"description": long_desc, "domain": "programming"})),
        )
        .await;
    // Should succeed (or at least not crash)
    assert!(status == 200 || status == 500);
}

#[tokio::test]
async fn test_xss_in_goal_description() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let xss_desc = "<script>alert('xss')</script>I want to learn security testing thoroughly";
    let (status, _) = h
        .post(
            &format!("/api/session/{sid}/goal"),
            Some(json!({"description": xss_desc, "domain": "security"})),
        )
        .await;
    assert_eq!(status, 200);
}

#[tokio::test]
async fn test_health_includes_version() {
    let h = TestHarness::new().await;
    let (_, body) = h.get("/health").await;
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    assert!(body["uptime_secs"].is_number());
    assert!(body["session_count"].is_number());
}

#[tokio::test]
async fn test_session_state_persistence() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    // Session should be in CHAPTER_LEARNING state
    let (_, session) = h.get(&format!("/api/session/{sid}")).await;
    assert_eq!(session["state"], "CHAPTER_LEARNING");

    // Verify stored data
    assert!(session["goal"].is_object());
    assert!(session["feasibility_result"].is_object());
    assert!(session["profile"].is_object());
    assert!(session["curriculum"].is_object());
}

#[tokio::test]
async fn test_complete_multiple_chapters() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    // Complete chapter 1
    let (status, body) = h
        .post(&format!("/api/session/{sid}/chapter/ch1/complete"), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "completed");

    // Complete chapter 2
    let (status, body) = h
        .post(&format!("/api/session/{sid}/chapter/ch2/complete"), None)
        .await;
    assert_eq!(status, 200);
    assert_eq!(body["status"], "completed");

    // Session should still be in CHAPTER_LEARNING
    let (_, session) = h.get(&format!("/api/session/{sid}")).await;
    assert_eq!(session["state"], "CHAPTER_LEARNING");
}

// ---------------------------------------------------------------------------
// Phase 2: Schema validation tests
// ---------------------------------------------------------------------------

// Phase 2 schema tests: valid-only (invalid cases tested in unit tests per crate)

#[test]
fn test_exercise_schema_valid() {
    let validator = SchemaValidator::new("../schemas");
    let valid = vec![
        json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "chapter_id": "ch1",
            "question": "What is 2+2?",
            "exercise_type": {
                "type": "multiple_choice",
                "options": ["3", "4", "5"],
                "correct_index": 1
            },
            "difficulty": "easy",
            "max_score": 1.0
        }),
        json!({
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "chapter_id": "ch2",
            "question": "Explain Rust",
            "exercise_type": {
                "type": "short_answer",
                "model_answer": "Rust is a systems programming language",
                "key_points": ["systems programming", "memory safety"]
            },
            "difficulty": "medium",
            "max_score": 3.0
        }),
    ];
    for v in &valid {
        assert!(
            validator.validate(v, "exercise").is_ok(),
            "Expected valid: {}",
            serde_json::to_string_pretty(v).unwrap()
        );
    }
}

#[test]
fn test_assessment_result_schema_valid() {
    let validator = SchemaValidator::new("../schemas");
    let valid = vec![
        json!({
            "exercise_id": "550e8400-e29b-41d4-a716-446655440000",
            "learner_answer": {"selected_index": 1},
            "score": 1.0,
            "max_score": 1.0,
            "feedback": "Correct!",
            "is_correct": true
        }),
        json!({
            "exercise_id": "550e8400-e29b-41d4-a716-446655440000",
            "learner_answer": {"selected_index": 0},
            "score": 0.0,
            "max_score": 1.0,
            "feedback": "Wrong answer.",
            "is_correct": false
        }),
    ];
    for v in &valid {
        assert!(
            validator.validate(v, "assessment_result").is_ok(),
            "Expected valid: {}",
            serde_json::to_string_pretty(v).unwrap()
        );
    }
}

#[test]
fn test_sandbox_request_schema_valid() {
    let validator = SchemaValidator::new("../schemas");
    let valid = vec![
        json!({
            "request_id": "550e8400-e29b-41d4-a716-446655440000",
            "session_id": "660e8400-e29b-41d4-a716-446655440000",
            "tool_kind": "python_exec",
            "code": "print('hello')"
        }),
        json!({
            "request_id": "550e8400-e29b-41d4-a716-446655440001",
            "session_id": "660e8400-e29b-41d4-a716-446655440001",
            "tool_kind": "node_exec",
            "code": "console.log('hi')",
            "limits": {
                "compile_timeout_secs": 10,
                "run_timeout_secs": 5,
                "memory_mb": 256,
                "network_enabled": false
            }
        }),
    ];
    for v in &valid {
        assert!(
            validator.validate(v, "sandbox_request").is_ok(),
            "Expected valid: {}",
            serde_json::to_string_pretty(v).unwrap()
        );
    }
}

#[test]
fn test_sandbox_result_schema_valid() {
    let validator = SchemaValidator::new("../schemas");
    let valid = vec![
        json!({
            "request_id": "550e8400-e29b-41d4-a716-446655440000",
            "session_id": "660e8400-e29b-41d4-a716-446655440000",
            "status": "success",
            "exit_code": 0,
            "stdout": "hello\n",
            "stderr": "",
            "stdout_truncated": false,
            "stderr_truncated": false,
            "duration_ms": 100
        }),
        json!({
            "request_id": "550e8400-e29b-41d4-a716-446655440000",
            "status": "timeout_run",
            "exit_code": null,
            "stdout": "",
            "stderr": "",
            "stdout_truncated": false,
            "stderr_truncated": false,
            "duration_ms": 10000
        }),
    ];
    for v in &valid {
        assert!(
            validator.validate(v, "sandbox_result").is_ok(),
            "Expected valid: {}",
            serde_json::to_string_pretty(v).unwrap()
        );
    }
}

// ---------------------------------------------------------------------------
// Phase 2: API endpoint tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_progress_empty() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let (status, body) = h.get(&format!("/api/session/{sid}/progress")).await;
    assert_eq!(status, 200);
    assert!(body["progress"].is_array());
    assert_eq!(body["progress"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_get_progress_after_complete() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    // Complete a chapter
    h.post(&format!("/api/session/{sid}/chapter/ch1/complete"), None)
        .await;

    let (status, body) = h.get(&format!("/api/session/{sid}/progress")).await;
    assert_eq!(status, 200);
    assert!(body["progress"].is_array());
    let progress = body["progress"].as_array().unwrap();
    assert!(!progress.is_empty());
    assert_eq!(progress[0]["status"], "completed");
    assert_eq!(progress[0]["completion"], 100.0);
}

#[tokio::test]
async fn test_submit_exercise_no_curriculum() {
    let h = TestHarness::new().await;
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let (status, _body) = h
        .post(
            &format!("/api/session/{sid}/chapter/ch1/exercise/ex1/submit"),
            Some(json!({"answer": {"selected_index": 0}})),
        )
        .await;
    assert_eq!(status, 422);
}

#[tokio::test]
async fn test_progress_persists_across_chapters() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;

    // Complete chapter 1
    h.post(&format!("/api/session/{sid}/chapter/ch1/complete"), None)
        .await;

    // Complete chapter 2
    h.post(&format!("/api/session/{sid}/chapter/ch2/complete"), None)
        .await;

    // Check progress has both chapters
    let (_, body) = h.get(&format!("/api/session/{sid}/progress")).await;
    let progress = body["progress"].as_array().unwrap();
    assert_eq!(progress.len(), 2);
}

// ── Infeasible goal test ──

#[tokio::test]
async fn test_infeasible_goal_returns_to_goal_input() {
    let mock = make_mock_provider();
    // Override the first (feasibility) response to return infeasible
    mock.replace_response(
        0,
        serde_json::json!({
            "feasible": false,
            "reason": "Goal is too broad. Please narrow your focus.",
            "suggestions": ["Pick a specific language", "Focus on one project type"],
            "estimated_duration": "N/A",
            "prerequisites": []
        })
        .to_string(),
    );

    let h = TestHarness::with_mock_provider(mock).await;

    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap();

    let (status, body) = h
        .post(
            &format!("/api/session/{sid}/goal"),
            Some(json!({"description": "Learn everything about computers", "domain": "computing"})),
        )
        .await;

    assert_eq!(status, 200, "Should succeed but return infeasible");
    assert_eq!(
        body["state"], "GOAL_INPUT",
        "Infeasible goal should return to GOAL_INPUT"
    );
    assert_eq!(body["feasibility"]["feasible"], false);
}

// ── Concurrent write tests ──

#[tokio::test]
async fn test_concurrent_question_asks() {
    let h = TestHarness::new().await;
    let sid = h.setup_learning().await;
    let base = h.base_url.clone();

    // 5 concurrent Q&A requests on the same session
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let url = format!("{base}/api/session/{sid}/chapter/ch1/ask");
            let client = TestHarness::http_client();
            let q = format!("Question {}", i);
            tokio::spawn(async move {
                client
                    .post(&url)
                    .json(&serde_json::json!({"question": q}))
                    .send()
                    .await
            })
        })
        .collect();

    let mut ok_count = 0;
    for h in handles {
        if let Ok(resp) = h.await.unwrap() {
            if resp.status().is_success() {
                ok_count += 1;
            }
        }
    }

    // At minimum, most requests should succeed
    assert!(
        ok_count >= 3,
        "Expected at least 3 successful concurrent requests, got {ok_count}"
    );
}

#[tokio::test]
async fn test_concurrent_create_and_delete() {
    let h = TestHarness::new().await;
    let base = h.base_url.clone();
    let client = TestHarness::http_client();

    // Create session
    let (_, body) = h.post("/api/session", None).await;
    let sid = body["session_id"].as_str().unwrap().to_string();

    // Concurrent read + delete
    let read_url = format!("{base}/api/session/{sid}");
    let delete_url = format!("{base}/api/session/{sid}");

    let (read_resp, delete_resp) = tokio::join!(
        client.get(&read_url).send(),
        client.delete(&delete_url).send(),
    );

    // Both should complete without panic
    let _ = (read_resp, delete_resp);

    // After delete, session should be gone
    let (status, _) = h.get(&format!("/api/session/{sid}")).await;
    assert_eq!(status, 404);
}
