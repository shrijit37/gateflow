//! Mock streaming upstream for local testing.
//!
//! `cargo run -p gateflow-server --example mock-upstream`
//!
//! Listens on :9099 and:
//! - `POST /chat`   — streams 20 OpenAI-style SSE chunks, one token each.
//! - `GET  /ping`   — plain JSON, for non-stream smoke tests.
//! - `POST /slow`   — emits the first chunk after `?delay_ms` (default 2000).

use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use futures::StreamExt as _;
use serde::Deserialize;
use tokio::time::sleep;
use tokio_stream::wrappers::IntervalStream;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("mock_upstream=info")
        .init();

    let app = Router::new()
        .route("/chat", post(chat))
        .route("/ping", get(ping))
        .route("/slow", post(slow))
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9099").await.unwrap();
    tracing::info!("mock upstream listening on http://0.0.0.0:9099");
    axum::serve(listener, app).await.unwrap();
}

async fn ping() -> impl IntoResponse {
    (StatusCode::OK, axum::Json(serde_json::json!({"ok": true})))
}

async fn chat() -> Response {
    let tick = tokio::time::interval(Duration::from_millis(40));
    let frames = IntervalStream::new(tick).take(20).map(|_| {
        let chunk = Bytes::from(
            "data: {\"choices\":[{\"delta\":{\"content\":\"a\"}}]}\n\n",
        );
        Ok::<_, std::convert::Infallible>(chunk)
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream; charset=utf-8")
        .header("x-mock-upstream", "true")
        .body(Body::from_stream(frames))
        .unwrap()
}

#[derive(Deserialize)]
struct SlowQuery {
    #[serde(default = "default_delay")]
    delay_ms: u64,
    #[serde(default = "default_chunks")]
    chunks: usize,
}

fn default_delay() -> u64 {
    2000
}
fn default_chunks() -> usize {
    5
}

async fn slow(Query(q): Query<SlowQuery>) -> Response {
    let delay = Duration::from_millis(q.delay_ms);
    let chunks = q.chunks;

    let stream = async_stream::stream! {
        sleep(delay).await;
        for i in 0..chunks {
            yield Ok::<_, std::convert::Infallible>(Bytes::from(format!(
                "data: {{\"i\":{i}}}\n\n"
            )));
            sleep(Duration::from_millis(100)).await;
        }
        yield Ok::<_, std::convert::Infallible>(Bytes::from("data: [DONE]\n\n"));
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream; charset=utf-8")
        .body(Body::from_stream(stream))
        .unwrap()
}