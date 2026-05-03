use std::sync::Arc;
use std::time::Duration;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent_core::{server, state, AppState, Config};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "agent_core=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();

    tracing::info!(
        host = %config.host,
        port = %config.port,
        model = %config.llm_model,
        prompts_dir = %config.prompts_dir.display(),
        schemas_dir = %config.schemas_dir.display(),
        data_dir = %config.data_dir.display(),
        max_sessions = config.max_sessions,
        session_ttl_hours = config.session_ttl_hours,
        "Starting agent-core"
    );

    let store = state::session::InMemorySessionStore::with_limit(config.max_sessions)
        .with_persistence(config.data_dir.clone());

    store.load_from_disk().await;

    store.start_eviction_task(
        Duration::from_secs(config.session_ttl_hours * 3600),
        Duration::from_secs(300),
    );

    // Build blup-agent config from environment, then overlay paths from
    // agent-core config so prompts/schemas/data dirs stay consistent.
    let mut agent_config = blup_agent::config::AgentConfig::from_env();
    agent_config.prompts_dir = config.prompts_dir.clone();
    agent_config.schemas_dir = config.schemas_dir.clone();
    agent_config.audit.storage_dir = config.data_dir.join("audit");
    agent_config.memory.storage_dir = config.data_dir.join("memory");

    let agent = Arc::new(
        blup_agent::AgentEngine::new(agent_config)
            .await
            .expect("Failed to create agent engine"),
    );

    // Initialize storage
    let storage_config =
        storage::config::StorageConfig::sqlite(&config.data_dir.join("blup.db").to_string_lossy());
    let storage = storage::Storage::connect(storage_config)
        .await
        .expect("Failed to connect to storage");
    storage
        .run_migrations()
        .await
        .expect("Failed to run storage migrations");

    // Initialize assessment engine
    let assessment = assessment_engine::AssessmentEngine::new();

    // Initialize content pipeline
    let content_pipeline = Arc::new(content_pipeline::ContentPipeline::new());

    // Initialize sandbox manager (use mock when BLUP_SANDBOX_MOCK=true)
    let sandbox_manager = if std::env::var("BLUP_SANDBOX_MOCK").as_deref() == Ok("true") {
        tracing::info!("Using MockExecutor for sandbox (BLUP_SANDBOX_MOCK=true)");
        let mut mock = sandbox_manager::MockExecutor::success_default();
        mock.set_response_fn(Box::new(|req| {
            use sandbox_manager::models::request::ToolKind;
            use sandbox_manager::models::result::SandboxResult;
            use sandbox_manager::models::status::ExecutionStatus;
            use sandbox_manager::models::result::ResourceUsage;

            let stdout = if req.tool_kind == ToolKind::TypstCompile {
                // Return a minimal valid PDF so the export flow works without Docker
                let pdf = b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>endobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000058 00000 n \n0000000115 00000 n \ntrailer\n<</Size 4/Root 1 0 R>>\nstartxref\n190\n%%EOF\n";
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(pdf)
            } else {
                "mock output\n".to_string()
            };

            SandboxResult {
                request_id: req.request_id,
                session_id: Some(req.session_id),
                status: ExecutionStatus::Success,
                exit_code: Some(0),
                stdout,
                stderr: String::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: 100,
                resource_usage: ResourceUsage::default(),
                error: None,
            }
        }));
        Arc::new(sandbox_manager::SandboxManager::with_executor(Box::new(
            mock,
        )))
    } else {
        let sandbox_config = sandbox_manager::SandboxConfig::default();
        Arc::new(sandbox_manager::SandboxManager::new(sandbox_config))
    };

    let app_state = AppState {
        config: Arc::new(config.clone()),
        store,
        agent,
        storage,
        assessment,
        content_pipeline,
        sandbox_manager,
    };

    let router = server::router::build_router(app_state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Listening on {}", addr);
    axum::serve(listener, router)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
            tracing::info!("Shutting down...");
        })
        .await?;

    Ok(())
}
