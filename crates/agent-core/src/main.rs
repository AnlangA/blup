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

    let app_state = AppState {
        config: Arc::new(config.clone()),
        store,
        agent,
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
