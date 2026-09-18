//! Gateflow gateway server entrypoint.

use gateflow_server::{config::Config, db::Db, node_registry::Registry, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Tracing first, so early errors are visible.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "gateflow=debug,tower_http=info,axum=info,h2=warn,hyper_util=warn".into()
            }),
        )
        .compact()
        .init();

    let config = Config::from_env();

    let db = Db::connect(&config.database_url).await?;
    tracing::info!(url = %config.database_url, "sqlite connected");

    let registry = Registry::new(&config)?;
    let state = AppState::new(config, db, registry);

    // Warm the hot-path cache with previously-deployed workflows.
    let deployed = state.db.deployed_rows().await?;
    for row in &deployed {
        match state.deploy(row).await {
            Ok(_) => tracing::info!(slug = %row.slug, "cache-warmed workflow"),
            Err(e) => {
                tracing::warn!(slug = %row.slug, error = %e, "failed to warm workflow (definition changed since last deploy?)")
            }
        }
    }

    gateflow_server::serve(state).await
}
