//! Locks the fleet health contract: `GET /health` returns
//! `{status, service, version, checks}` and its HTTP status agrees with
//! `status`. A drift here silently breaks every deploy gate that waits on it.

use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use gateflow_server::config::Config;
use gateflow_server::node_registry::Registry;
use gateflow_server::state::AppState;
use gateflow_server::{build_router, db};
use tower::ServiceExt;

async fn state() -> AppState {
    let mut config = Config::from_env();
    // In-memory so the test never touches the repo's gateflow.db.
    config.database_url = "sqlite::memory:".to_string();
    let handle = db::Db::connect(&config.database_url).await.expect("db");
    let registry = Registry::new(&config).expect("registry");
    AppState::new(config, handle, registry)
}

#[tokio::test]
async fn health_matches_fleet_contract() {
    let app = build_router(state().await);
    let resp = app
        .oneshot(
            Request::get("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .expect("router response");

    assert_eq!(resp.status(), StatusCode::OK, "healthy db must answer 200");

    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();

    let obj = body.as_object().expect("object");
    let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["checks", "service", "status", "version"],
        "health shape must be exactly the fleet contract"
    );
    assert_eq!(body["service"], "gateflow-api");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["checks"]["database"], "up");
    assert!(body["version"].is_string(), "version must be a string");
}

#[tokio::test]
async fn health_reports_503_when_database_is_unreachable() {
    let state = state().await;
    state.db.pool.close().await;
    let app = build_router(state);

    let resp = app
        .oneshot(
            Request::get("/health")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .expect("router response");

    // A healthy-looking 200 over a dead database is the failure mode this
    // endpoint exists to prevent.
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["status"], "down");
    assert_eq!(body["checks"]["database"], "down");
}
