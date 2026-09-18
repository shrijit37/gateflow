//! Gateflow server library: router construction (bin + integration tests).

pub mod config;
pub mod dataplane;
pub mod db;
pub mod models;
pub mod node_registry;
pub mod routes;
pub mod state;

use axum::Router;
use state::AppState;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;

/// Build the full HTTP router (control-plane + data-plane).
pub fn build_router(state: AppState) -> Router {
    let origins: Vec<axum::http::HeaderValue> = state
        .config
        .allowed_origins
        .iter()
        .filter_map(|o| axum::http::HeaderValue::from_str(o).ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(Any)
        .allow_headers(Any);

    let body_limit_bytes = state.config.body_limit_bytes;

    Router::new()
        // ---- control-plane ----
        .route("/healthz", axum::routing::get(routes::healthz))
        .route(
            "/api/workflows",
            axum::routing::get(routes::list_workflows).post(routes::create_workflow),
        )
        .route(
            "/api/workflows/{id}",
            axum::routing::get(routes::get_workflow).put(routes::update_workflow),
        )
        .route(
            "/api/workflows/{id}/deploy",
            axum::routing::post(routes::deploy_workflow),
        )
        // ---- data-plane: the streaming hot path ----
        .route("/flow/{slug}", axum::routing::any(dataplane::flow_handler))
        .with_state(state)
        .layer(cors)
        .layer(RequestBodyLimitLayer::new(body_limit_bytes))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(
            TraceLayer::new_for_http().make_span_with(
                tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO),
            ),
        )
}

/// Spawn serving + graceful shutdown given a listener.
pub async fn serve(state: AppState) -> anyhow::Result<()> {
    use tokio::net::TcpListener;

    let bind = state.config.bind.clone();
    let app = build_router(state);
    let listener = TcpListener::bind(&bind).await?;

    tracing::info!(addr = %bind, "gateflow listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!(error = %e, "failed to listen for ctrl-c");
        }
    };
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
