use std::sync::Arc;

use agent_core::llm::client::LlmClient;
use agent_core::prompts::loader::PromptLoader;
use agent_core::state::session::InMemorySessionStore;
use agent_core::validation::schema_validator::SchemaValidator;
use agent_core::{AppState, Config};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde_json::json;
use tokio::sync::Mutex;

// ── Mock LLM Gateway ──

#[derive(Clone)]
pub struct MockGatewayState {
    pub responses: Arc<Mutex<Vec<String>>>,
}

async fn mock_complete(
    State(state): State<MockGatewayState>,
    Json(body): Json<serde_json::Value>,
) -> axum::response::Response {
    let messages = body["messages"].as_array().unwrap();
    let user_content = messages
        .iter()
        .find(|m| m["role"] == "user")
        .map(|m| m["content"].as_str().unwrap_or(""))
        .unwrap_or("");
    let system_content = messages
        .iter()
        .find(|m| m["role"] == "system")
        .map(|m| m["content"].as_str().unwrap_or(""))
        .unwrap_or("");

    let content: serde_json::Value = if system_content.contains("Feasibility Check") {
        json!({"feasible":true,"reason":"Well-defined and achievable.","suggestions":[],"estimated_duration":"4-6 weeks","prerequisites":["basic computer literacy"]})
    } else if system_content.contains("Profile Collection") || system_content.contains("collection")
    {
        json!({"experience_level":{"domain_knowledge":"beginner"},"learning_style":{"preferred_format":["text"]},"available_time":{"hours_per_week":10}})
    } else if system_content.contains("Curriculum Planning") || system_content.contains("planning")
    {
        json!({"title":"Python Fundamentals","description":"A comprehensive introduction","chapters":[{"id":"ch1","title":"Getting Started","order":1,"objectives":["Install Python"],"estimated_minutes":30},{"id":"ch2","title":"Variables","order":2,"objectives":["Understand variables"],"estimated_minutes":45}],"estimated_duration":"4-6 weeks"})
    } else if system_content.contains("Chapter Teaching") || system_content.contains("teaching") {
        json!({"content":"# Chapter Content\n\nLearning content in markdown."})
    } else if system_content.contains("Question Answering") || system_content.contains("answering")
    {
        json!({"content":"This is the answer to your question."})
    } else {
        json!({"feasible":true,"reason":"Looks good.","suggestions":[],"estimated_duration":"2 weeks","prerequisites":[]})
    };

    {
        let mut history = state.responses.lock().await;
        history.push(user_content.to_string());
    }

    let is_stream = body["stream"].as_bool().unwrap_or(false);

    if is_stream {
        // Return SSE-formatted streaming response
        let json_str = content.to_string();
        let bytes = json_str.as_bytes();
        let chunk_size = 16usize;
        let chunks: Vec<&[u8]> = bytes.chunks(chunk_size).collect();

        let sse_body = chunks
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let text = String::from_utf8_lossy(c);
                let chunk_json = json!({
                    "content": text,
                    "index": i as u32,
                });
                format!("data: {}\n\n", serde_json::to_string(&chunk_json).unwrap())
            })
            .collect::<Vec<_>>()
            .join("")
            + "data: {}\n\n";

        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/event-stream")
            .body(axum::body::Body::from(sse_body))
            .unwrap()
    } else {
        let body = axum::body::Body::from(
            serde_json::to_string(&json!({
                "content": content.to_string(),
                "model": "mock-model",
                "provider": "mock",
                "usage": {"prompt_tokens":100,"completion_tokens":50,"total_tokens":150},
                "finish_reason": "stop"
            }))
            .unwrap(),
        );
        axum::response::Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json")
            .body(body)
            .unwrap()
    }
}

// ── Test Harness ──

pub struct TestHarness {
    pub base_url: String,
    http: reqwest::Client,
    _gateway: tokio::task::JoinHandle<()>,
    _server: tokio::task::JoinHandle<()>,
}

impl TestHarness {
    pub async fn new() -> Self {
        // Start mock LLM gateway
        let gw_state = MockGatewayState {
            responses: Arc::new(Mutex::new(Vec::new())),
        };
        let gw_app = Router::new()
            .route("/v1/gateway/complete", post(mock_complete))
            .with_state(gw_state);
        let gw_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gw_addr = gw_listener.local_addr().unwrap();
        let gw_url = format!("http://{}", gw_addr);
        let gw_handle = tokio::spawn(async move {
            axum::serve(gw_listener, gw_app).await.unwrap();
        });

        // Start the agent-core server
        let config = Config {
            llm_gateway_url: gw_url.clone(),
            ..Default::default()
        };
        let store = InMemorySessionStore::new();
        let llm = LlmClient::new(gw_url, String::new());
        let prompt_loader = PromptLoader::new("../prompts");
        let validator = SchemaValidator::new("../schemas");

        let app_state = AppState {
            config: Arc::new(config),
            store,
            llm,
            prompts: Arc::new(prompt_loader),
            validator: Arc::new(validator),
        };

        let app = agent_core::server::router::build_router(app_state);
        let app_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let app_addr = app_listener.local_addr().unwrap();
        let base_url = format!("http://{}", app_addr);

        let server_handle = tokio::spawn(async move {
            axum::serve(app_listener, app).await.unwrap();
        });

        let http = reqwest::Client::new();

        Self {
            base_url,
            http,
            _gateway: gw_handle,
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
                    Some(json!({"question_id": format!("q{i}"), "answer": *ans})),
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
            Some(json!({"description": "Learn Python for data analysis", "domain": "programming"})),
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
            _ => self.http.get(&url),
        };

        if let Some(b) = body {
            req = req.json(&b);
        }

        match req.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let body: serde_json::Value = resp.json().await.unwrap_or(json!({}));
                (status, body)
            }
            Err(e) => {
                eprintln!("Request error: {e}");
                (0, json!({"error": e.to_string()}))
            }
        }
    }
}
