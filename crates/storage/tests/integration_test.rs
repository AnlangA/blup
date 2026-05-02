use storage::config::StorageConfig;
use storage::models::assessment::AssessmentInput;
use storage::Storage;

fn memory_config() -> StorageConfig {
    StorageConfig::sqlite(":memory:")
}

#[tokio::test]
async fn test_storage_creation() {
    let config = memory_config();
    let storage = Storage::connect(config).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    assert_eq!(session.state, "IDLE");
    assert!(session.goal.is_none());
}

#[tokio::test]
async fn test_session_lifecycle() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let session_id = uuid::Uuid::parse_str(&session.id).unwrap();

    storage
        .update_session_state(session_id, "GOAL_INPUT")
        .await
        .unwrap();

    storage
        .save_goal(
            session_id,
            serde_json::json!({"description": "Learn Rust", "domain": "programming"}),
        )
        .await
        .unwrap();

    let retrieved = storage.get_session(session_id).await.unwrap().unwrap();
    assert_eq!(retrieved.state, "GOAL_INPUT");
    assert!(retrieved.goal.is_some());

    let sessions = storage.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);

    storage.delete_session(session_id).await.unwrap();
    assert_eq!(storage.list_sessions().await.unwrap().len(), 0);
}

#[tokio::test]
async fn test_create_session_with_specific_id() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let id = uuid::Uuid::new_v4();
    let session = storage.create_session_with_id(id).await.unwrap();
    assert_eq!(session.id, id.to_string());

    let retrieved = storage.get_session(id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, id.to_string());
}

#[tokio::test]
async fn test_save_and_get_user_profile() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let profile = serde_json::json!({
        "experience_level": {"domain_knowledge": "intermediate"},
        "learning_style": {"preferred_format": ["text", "interactive"]},
        "available_time": {"hours_per_week": 10.0}
    });
    storage.save_user_profile(sid, profile).await.unwrap();

    let retrieved = storage.get_session(sid).await.unwrap().unwrap();
    assert!(retrieved.user_profile.is_some());
}

#[tokio::test]
async fn test_message_operations() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    // Save multiple messages
    for (i, role) in ["user", "assistant", "user"].iter().enumerate() {
        storage
            .save_message(sid, Some("ch1"), role, &format!("Message {i}"))
            .await
            .unwrap();
    }

    // Get messages — most recent first
    let messages = storage.get_messages(sid, 10, None).await.unwrap();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0]["role"], "user"); // last inserted = first in DESC order
}

#[tokio::test]
async fn test_message_pagination_with_before() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    storage
        .save_message(sid, Some("ch1"), "user", "msg1")
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    let checkpoint = chrono::Utc::now();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    storage
        .save_message(sid, Some("ch1"), "assistant", "msg2")
        .await
        .unwrap();

    // Get messages before checkpoint — should only get msg1
    let messages = storage
        .get_messages(sid, 10, Some(checkpoint))
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
}

#[tokio::test]
async fn test_progress_upsert_and_read() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let progress = serde_json::json!({
        "status": "in_progress",
        "completion": 45.0,
        "time_spent_minutes": 20
    });
    storage.upsert_progress(sid, "ch1", progress).await.unwrap();

    // Update the same record
    let updated = serde_json::json!({
        "status": "completed",
        "completion": 100.0,
        "time_spent_minutes": 60
    });
    storage.upsert_progress(sid, "ch1", updated).await.unwrap();

    let retrieved = storage.get_progress(sid, "ch1").await.unwrap().unwrap();
    assert_eq!(retrieved["status"], "completed");
    assert_eq!(retrieved["completion"], 100.0);
}

#[tokio::test]
async fn test_assessment_operations() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let exercise = serde_json::json!({"id": "ex1", "question": "What is 2+2?", "type": "mc"});
    let answer = serde_json::json!({"selected_index": 1});
    let evaluation = serde_json::json!({"score": 1.0, "is_correct": true});

    storage
        .save_assessment(
            sid,
            Some("ch1"),
            &AssessmentInput {
                exercise,
                answer: Some(answer),
                evaluation: Some(evaluation),
                score: Some(1.0),
                max_score: Some(1.0),
            },
        )
        .await
        .unwrap();

    let assessments = storage.get_assessments(sid).await.unwrap();
    assert_eq!(assessments.len(), 1);
    assert_eq!(assessments[0]["score"], 1.0);
}

#[tokio::test]
async fn test_large_goal_description() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    // 10KB goal description
    let large_desc = "Learn ".to_string() + &"x".repeat(10000);
    storage
        .save_goal(
            sid,
            serde_json::json!({"description": large_desc, "domain": "testing"}),
        )
        .await
        .unwrap();

    let retrieved = storage.get_session(sid).await.unwrap().unwrap();
    assert!(retrieved.goal.is_some());
}

#[tokio::test]
async fn test_unicode_content_in_messages() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let unicode_content = "Hello 世界 🌍 — こんにちは";
    storage
        .save_message(sid, Some("ch1"), "assistant", unicode_content)
        .await
        .unwrap();

    let messages = storage.get_messages(sid, 1, None).await.unwrap();
    assert_eq!(messages[0]["content"], unicode_content);
}

#[tokio::test]
async fn test_nonexistent_session_returns_none() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let result = storage.get_session(uuid::Uuid::new_v4()).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_delete_nonexistent_session_is_error() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let result = storage.delete_session(uuid::Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_backup_and_restore() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let backup_path = tmp.path().join("backup.db");

    // Create and populate a database
    let config = StorageConfig::sqlite(&db_path.to_string_lossy());
    let storage = Storage::connect(config.clone()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();
    storage
        .save_goal(
            sid,
            serde_json::json!({"description":"Learn Rust","domain":"programming"}),
        )
        .await
        .unwrap();
    storage
        .update_session_state(sid, "GOAL_INPUT")
        .await
        .unwrap();

    // Backup
    storage
        .backup(&backup_path.to_string_lossy())
        .await
        .unwrap();
    assert!(backup_path.exists());

    // Restore into a new database
    let restored = Storage::restore(&config, &backup_path.to_string_lossy())
        .await
        .unwrap();
    restored.run_migrations().await.unwrap();

    let sessions = restored.list_sessions().await.unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].state, "GOAL_INPUT");
    assert!(sessions[0].goal.is_some());
}

// ── Phase 2: Additional concurrent and edge case tests ──

#[tokio::test]
async fn test_concurrent_session_creation() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let mut handles = vec![];
    for _ in 0..10 {
        let storage_clone = storage.clone();
        handles.push(tokio::spawn(async move {
            storage_clone.create_session().await.unwrap()
        }));
    }

    let mut sessions = vec![];
    for handle in handles {
        sessions.push(handle.await.unwrap());
    }

    // All sessions should have unique IDs
    let mut ids: Vec<String> = sessions.iter().map(|s| s.id.clone()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 10);
}

#[tokio::test]
async fn test_concurrent_message_writes() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let mut handles = vec![];
    for i in 0..5 {
        let storage_clone = storage.clone();
        handles.push(tokio::spawn(async move {
            storage_clone
                .save_message(sid, Some("ch1"), "user", &format!("Message {i}"))
                .await
                .unwrap()
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let messages = storage.get_messages(sid, 10, None).await.unwrap();
    assert_eq!(messages.len(), 5);
}

#[tokio::test]
async fn test_concurrent_progress_updates() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let mut handles = vec![];
    for i in 0..5 {
        let storage_clone = storage.clone();
        handles.push(tokio::spawn(async move {
            let progress = serde_json::json!({
                "status": "in_progress",
                "completion": (i as f64 * 20.0),
                "time_spent_minutes": i * 10
            });
            storage_clone
                .upsert_progress(sid, &format!("ch{}", i), progress)
                .await
                .unwrap()
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let all_progress = storage.get_all_progress(sid).await.unwrap();
    assert_eq!(all_progress.len(), 5);
}

#[tokio::test]
async fn test_large_message_content() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    // 100KB message
    let large_content = "x".repeat(100 * 1024);
    storage
        .save_message(sid, Some("ch1"), "assistant", &large_content)
        .await
        .unwrap();

    let messages = storage.get_messages(sid, 1, None).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["content"], large_content);
}

#[tokio::test]
async fn test_session_state_transitions() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let states = vec![
        "GOAL_INPUT",
        "FEASIBILITY_CHECK",
        "PROFILE_COLLECTION",
        "CURRICULUM_PLANNING",
        "CHAPTER_LEARNING",
        "COMPLETED",
    ];

    for state in states {
        storage.update_session_state(sid, state).await.unwrap();
        let retrieved = storage.get_session(sid).await.unwrap().unwrap();
        assert_eq!(retrieved.state, state);
    }
}

#[tokio::test]
async fn test_curriculum_with_nested_data() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let curriculum = serde_json::json!({
        "title": "Advanced Rust",
        "description": "Deep dive into Rust",
        "chapters": [
            {
                "id": "ch1",
                "title": "Ownership",
                "order": 1,
                "objectives": ["Understand ownership", "Learn borrowing"],
                "content": "# Ownership\n\nRust's ownership system...",
                "exercises": [
                    {"type": "multiple_choice", "question": "What is ownership?"},
                    {"type": "coding", "question": "Fix the code"}
                ]
            },
            {
                "id": "ch2",
                "title": "Lifetimes",
                "order": 2,
                "objectives": ["Understand lifetimes"],
                "prerequisites": ["ch1"]
            }
        ],
        "metadata": {
            "difficulty": "advanced",
            "estimated_hours": 40
        }
    });

    storage
        .save_curriculum(sid, curriculum.clone())
        .await
        .unwrap();

    let retrieved = storage.get_curriculum(sid).await.unwrap().unwrap();
    assert_eq!(retrieved["title"], "Advanced Rust");
    assert_eq!(retrieved["chapters"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_assessment_with_complex_evaluation() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    let session = storage.create_session().await.unwrap();
    let sid = uuid::Uuid::parse_str(&session.id).unwrap();

    let input = storage::models::assessment::AssessmentInput {
        exercise: serde_json::json!({
            "id": "ex1",
            "type": "coding",
            "language": "python",
            "test_cases": [
                {"input": "2,3", "expected": "5"},
                {"input": "0,0", "expected": "0"}
            ]
        }),
        answer: Some(serde_json::json!({
            "code": "def add(a, b): return a + b"
        })),
        evaluation: Some(serde_json::json!({
            "score": 2.0,
            "max_score": 2.0,
            "feedback": "All tests passed",
            "test_results": [
                {"name": "test1", "passed": true},
                {"name": "test2", "passed": true}
            ]
        })),
        score: Some(2.0),
        max_score: Some(2.0),
    };

    storage
        .save_assessment(sid, Some("ch1"), &input)
        .await
        .unwrap();

    let assessments = storage.get_assessments(sid).await.unwrap();
    assert_eq!(assessments.len(), 1);
    assert_eq!(assessments[0]["score"], 2.0);
}

#[tokio::test]
async fn test_rollback_multiple_steps() {
    let storage = Storage::connect(memory_config()).await.unwrap();
    storage.run_migrations().await.unwrap();

    // Rollback all 5 migrations
    storage.rollback(5).await.unwrap();

    // Re-migrate should work
    storage.run_migrations().await.unwrap();

    // Should be able to create sessions again
    let session = storage.create_session().await.unwrap();
    assert_eq!(session.state, "IDLE");
}
