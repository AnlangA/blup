use std::sync::Arc;
use std::time::Duration;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use agent_core::{llm, prompts, server, state, validation, AppState, Config};

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
        gateway_url = %config.llm_gateway_url,
        prompts_dir = %config.prompts_dir.display(),
        schemas_dir = %config.schemas_dir.display(),
        data_dir = %config.data_dir.display(),
        max_sessions = config.max_sessions,
        session_ttl_hours = config.session_ttl_hours,
        "Starting agent-core"
    );

    let store = state::session::InMemorySessionStore::with_limit(config.max_sessions)
        .with_persistence(config.data_dir.clone());

    // Restore persisted sessions
    store.load_from_disk().await;

    // Start background eviction of stale sessions
    store.start_eviction_task(
        Duration::from_secs(config.session_ttl_hours * 3600),
        Duration::from_secs(300), // check every 5 minutes
    );

    let llm = llm::client::LlmClient::new(
        config.llm_gateway_url.clone(),
        config.llm_gateway_secret.clone(),
    );
    let prompt_loader = prompts::loader::PromptLoader::new(&config.prompts_dir);
    let validator = validation::schema_validator::SchemaValidator::new(&config.schemas_dir);

    let app_state = AppState {
        config: Arc::new(config.clone()),
        store,
        llm,
        prompts: Arc::new(prompt_loader),
        validator: Arc::new(validator),
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
