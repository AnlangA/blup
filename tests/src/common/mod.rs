use std::sync::Arc;

use agent_core::state::session::InMemorySessionStore;
use agent_core::{AppState, Config};
use blup_agent::config::AgentConfig;
use blup_agent::prompt::PromptLoader;
use blup_agent::provider::mock::MockProvider;
use blup_agent::schema::SchemaValidator;
use blup_agent::AgentEngine;

/// Create a mock provider preloaded with enough valid responses for the full
/// learning flow. Each integration test gets a fresh instance.
pub fn make_mock_provider() -> MockProvider {
    let mock = MockProvider::new();
    // Response 1: feasibility check
    mock.push_response(
        serde_json::json!({
            "feasible": true,
            "reason": "This is a well-defined learning goal.",
            "suggestions": ["Start with fundamentals", "Practice regularly"],
            "estimated_duration": "4 weeks",
            "prerequisites": ["Basic computer skills"]
        })
        .to_string(),
    );
    // Response 2-3: profile collection follow-up questions
    mock.push_response(
        serde_json::json!({
            "next_question": "What learning format works best for you?"
        })
        .to_string(),
    );
    mock.push_response(
        serde_json::json!({
            "next_question": "How much time can you dedicate each week?"
        })
        .to_string(),
    );
    // Response 4: profile collection (final round)
    mock.push_response(
        serde_json::json!({
            "experience_level": {
                "domain_knowledge": "beginner",
                "related_domains": [],
                "years_of_experience": 0.0
            },
            "learning_style": {
                "preferred_format": ["text", "interactive"],
                "pace_preference": "moderate"
            },
            "available_time": {
                "hours_per_week": 10.0,
                "preferred_session_length_minutes": 60.0
            },
            "goals": {
                "primary_goal": "Learn Python for data analysis",
                "secondary_goals": [],
                "success_criteria": "Build a data pipeline"
            },
            "preferences": {
                "language": "en",
                "difficulty_bias": "standard",
                "feedback_frequency": "end_of_section"
            }
        })
        .to_string(),
    );
    // Response 5: curriculum plan
    mock.push_response(
        serde_json::json!({
            "title": "Python Data Analysis",
            "description": "A structured curriculum for learning Python data analysis",
            "chapters": [
                {
                    "id": "ch1",
                    "title": "Python Fundamentals",
                    "order": 1,
                    "objectives": ["Install Python", "Write basic programs"],
                    "prerequisites": [],
                    "estimated_minutes": 60
                },
                {
                    "id": "ch2",
                    "title": "Data Structures",
                    "order": 2,
                    "objectives": ["Understand lists", "Work with dictionaries"],
                    "prerequisites": ["ch1"],
                    "estimated_minutes": 90
                }
            ],
            "estimated_duration": "4 weeks",
            "prerequisites_summary": [],
            "learning_objectives": ["Write Python scripts", "Analyze data"]
        })
        .to_string(),
    );
    // Response 6+: chapter teaching / Q&A / repair text responses
    for _ in 0..10 {
        mock.push_response("# Chapter Content\n\nThis is the chapter teaching content with explanations and examples.\n\n## Key Concepts\n\n- Concept 1\n- Concept 2\n\n## Examples\n\nHere are some practical examples.");
    }
    mock
}

/// Test harness that spins up the agent-core server with a MockProvider.
pub struct TestHarness {
    pub base_url: String,
    http: reqwest::Client,
    _server: tokio::task::JoinHandle<()>,
}

impl TestHarness {
    pub async fn new() -> Self {
        Self::build(make_mock_provider()).await
    }

    /// Create a harness with a custom mock provider (to test infeasible goals, etc.).
    pub async fn with_mock_provider(mock: MockProvider) -> Self {
        Self::build(mock).await
    }

    async fn build(mock: MockProvider) -> Self {
        let config = Config {
            ..Default::default()
        };

        let store = InMemorySessionStore::new();

        let agent_config = AgentConfig {
            provider: blup_agent::config::ProviderConfig {
                provider_type: blup_agent::config::ProviderType::Mock,
                model: "mock-model".to_string(),
                ..Default::default()
            },
            prompts_dir: std::path::PathBuf::from("../prompts"),
            schemas_dir: std::path::PathBuf::from("../schemas"),
            audit: blup_agent::config::AuditConfig {
                enabled: false,
                storage_dir: std::path::PathBuf::from("/tmp/blup-test-audit"),
            },
            memory: blup_agent::config::MemoryConfig {
                storage_dir: std::path::PathBuf::from("/tmp/blup-test-memory"),
                ..Default::default()
            },
            ..Default::default()
        };

        let prompts = Arc::new(PromptLoader::new(&agent_config.prompts_dir));
        let validator = Arc::new(SchemaValidator::new(&agent_config.schemas_dir));

        let agent = Arc::new(
            AgentEngine::with_provider(Arc::new(mock), prompts, validator, agent_config).await,
        );

        let storage_config = storage::config::StorageConfig::sqlite(":memory:");
        let storage = storage::Storage::connect(storage_config).await.unwrap();
        storage.run_migrations().await.unwrap();

        let assessment = assessment_engine::AssessmentEngine::new();

        let content_pipeline = Arc::new(content_pipeline::ContentPipeline::new());
        let sandbox_manager = Arc::new(sandbox_manager::SandboxManager::with_executor(Box::new(
            sandbox_manager::MockExecutor::success_default(),
        )));

        let app_state = AppState {
            config: Arc::new(config),
            store,
            agent,
            storage,
            assessment,
            content_pipeline,
            sandbox_manager,
        };

        let app = agent_core::server::router::build_router(app_state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        let server_handle = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let http = reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to build HTTP client");

        Self {
            base_url,
            http,
            _server: server_handle,
        }
    }

    pub async fn post(
        &self,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> (u16, serde_json::Value) {
        self.request("POST", path, body).await
    }

    pub async fn get(&self, path: &str) -> (u16, serde_json::Value) {
        self.request("GET", path, None).await
    }

    pub async fn delete(&self, path: &str) -> (u16, serde_json::Value) {
        self.request("DELETE", path, None).await
    }

    /// Create a new reqwest client configured for test use (no proxy).
    pub fn http_client() -> reqwest::Client {
        reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("Failed to build HTTP client")
    }

    /// Submit 3 profile answers to complete the profile collection flow.
    pub async fn complete_profile(&self, sid: &str) {
        let answers = [
            "No experience at all",
            "Reading text and documentation",
            "2-5 hours per week",
        ];
        for (i, ans) in answers.iter().enumerate() {
            let (status, _body) = self
                .post(
                    &format!("/api/session/{sid}/profile/answer"),
                    Some(serde_json::json!({"question_id": format!("q{i}"), "answer": *ans})),
                )
                .await;
            assert_eq!(status, 200, "Profile answer {i} failed");
        }
    }

    /// Full setup: create session, submit goal, complete profile, get curriculum.
    /// Returns session_id. Session state is CHAPTER_LEARNING after this.
    pub async fn setup_learning(&self) -> String {
        let (_, body) = self.post("/api/session", None).await;
        let sid = body["session_id"].as_str().unwrap().to_string();

        self.post(
            &format!("/api/session/{sid}/goal"),
            Some(serde_json::json!({"description": "Learn Python for data analysis", "domain": "programming"})),
        )
        .await;

        self.complete_profile(&sid).await;

        self.get(&format!("/api/session/{sid}/curriculum")).await;
        sid
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> (u16, serde_json::Value) {
        let url = format!("{}{}", self.base_url, path);
        let mut req = match method {
            "GET" => self.http.get(&url),
            "POST" => self.http.post(&url),
            "DELETE" => self.http.delete(&url),
            _ => self.http.get(&url),
        };

        if let Some(b) = body {
            req = req.json(&b);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body: serde_json::Value = resp.json().await.unwrap_or(serde_json::json!({}));
                (status, body)
            }
            Err(e) => {
                eprintln!("Request error: {e}");
                (0, serde_json::json!({"error": e.to_string()}))
            }
        }
    }
}
