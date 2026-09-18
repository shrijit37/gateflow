//! Data-plane: `ANY /flow/{slug}` — the streaming hot path.
//!
//! Flow: route -> ingress method check -> tiny body prefix peek (for protocol
//! detection only) -> run compiled DAG with the body as input frames -> mine
//! the first frame (Head) to set the real response head -> stream everything
//! after that straight to the client, flushing as it arrives.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::{HeaderMap, Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures::StreamExt;
use gateflow_engine::{ExecCtx, NodeError, Protocol, StreamFrame};
use gateflow_protocol::{DetectHints, detect};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::db::Db;
use crate::models::ExecutionRecord;
use crate::state::{AppState, Deployed};

pub async fn flow_handler(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    req: Request<Body>,
) -> Response {
    match handle_flow(&state, &slug, req).await {
        Ok(resp) => resp,
        Err(err) => {
            error!(target: "gateflow::dataplane", slug, error = %err, "flow request failed");
            json_error(StatusCode::BAD_GATEWAY, &err)
        }
    }
}

async fn handle_flow(state: &AppState, slug: &str, req: Request<Body>) -> Result<Response, String> {
    let (parts, body) = req.into_parts();
    let method = parts.method.clone();
    let headers = parts.headers.clone();
    let path = parts.uri.path().to_string();

    let Some(deployed) = state.cache.read().await.get(slug).cloned() else {
        return Err(format!("no deployed workflow at /flow/{slug}"));
    };

    // Ingress method gate (gateway never handles OPTIONS itself, but be safe).
    let meta = &deployed.ingress;
    if !meta.any_method && meta.method != method {
        return Ok(json_error(
            StatusCode::METHOD_NOT_ALLOWED,
            &format!("ingress accepts {} only", meta.method),
        ));
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    let abort = CancellationToken::new();

    // Peek a small prefix of the body purely for protocol detection.
    let (prefix, body) = peek_prefix(body, state.config.peek_prefix_bytes, meta.timeout).await;
    let protocol = resolve_protocol(&deployed, &path, &headers, &prefix);

    let ctx = ExecCtx::new(
        request_id.clone(),
        &deployed.exe.workflow_id,
        &deployed.exe.workflow_slug,
        protocol,
        headers.clone(),
        abort,
    )
    .with_deadline(Some(tokio::time::Instant::now() + meta.timeout));

    // Body -> frames (counting bytes in).
    let ctx_counter = ctx.clone();
    let body_frames: gateflow_engine::FrameStream =
        Box::pin(body.into_data_stream().map(move |chunk| match chunk {
            Ok(b) => {
                ctx_counter
                    .bytes_in
                    .fetch_add(b.len() as u64, Ordering::Relaxed);
                Ok(StreamFrame::Raw(b))
            }
            Err(e) => Err(NodeError::Node {
                node: "body".into(),
                msg: e.to_string(),
            }),
        }));

    let out = deployed.exe.execute(ctx.clone(), body_frames);
    let recorder = ExecutionRecorder::new(state.db.clone(), &ctx, &deployed.exe.workflow_id, slug);

    // First frame carries the upstream head (or an error).
    let connect_timeout = state.config.default_connect_timeout;
    let (first, tail) = tokio::time::timeout(connect_timeout, out.into_future())
        .await
        .map_err(|_| format!("upstream connect timeout after {connect_timeout:?}"))?;

    match first {
        Some(Ok(StreamFrame::Head(head))) => {
            recorder.set_head(head.status);
            let mut builder = Response::builder().status(head.status);
            for (name, value) in head.headers {
                // Never forward hop-by-hop headers.
                if name.eq_ignore_ascii_case("transfer-encoding")
                    || name.eq_ignore_ascii_case("connection")
                    || name.eq_ignore_ascii_case("keep-alive")
                    || name.eq_ignore_ascii_case("content-length")
                {
                    continue;
                }
                if let Ok(hname) = header::HeaderName::from_bytes(name.as_bytes())
                    && let Ok(hvalue) = header::HeaderValue::from_str(&value)
                {
                    builder = builder.header(hname, hvalue);
                }
            }
            builder
                .body(Body::from_stream(frames_to_body(tail, recorder)))
                .map_err(|_| "failed to build streaming response".to_string())
        }
        Some(Ok(StreamFrame::Raw(_) | StreamFrame::Event { .. } | StreamFrame::End)) => {
            // No upstream head (non-egress terminal): default streaming content type.
            recorder.set_head(200);
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/event-stream; charset=utf-8")
                .header(header::CACHE_CONTROL, "no-cache")
                .body(Body::from_stream(frames_to_body(tail, recorder)))
                .unwrap())
        }
        Some(Err(e)) => {
            recorder.set_error("upstream_error");
            Err(format!("upstream error before response: {e}"))
        }
        None => {
            recorder.set_error("upstream_empty");
            Err("upstream closed the connection without responding".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Execution logging
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct ExecutionRecorder {
    db: Db,
    request_id: String,
    workflow_id: String,
    slug: String,
    protocol: Option<String>,
    started_at: String,
    started: tokio::time::Instant,
    status_code: Arc<Mutex<Option<u16>>>,
    status: Arc<Mutex<Option<&'static str>>>,
    bytes_in: Arc<AtomicU64>,
    bytes_out: Arc<AtomicU64>,
    done: Arc<Mutex<bool>>,
}

impl ExecutionRecorder {
    fn new(db: Db, ctx: &ExecCtx, workflow_id: &str, slug: &str) -> Self {
        Self {
            db,
            request_id: ctx.request_id.clone(),
            workflow_id: workflow_id.to_string(),
            slug: slug.to_string(),
            protocol: Some(ctx.protocol.as_str().to_string()),
            started_at: utc_rfc3339_now(),
            started: ctx.started_at,
            status_code: Arc::new(Mutex::new(None)),
            status: Arc::new(Mutex::new(None)),
            bytes_in: ctx.bytes_in.clone(),
            bytes_out: ctx.bytes_out.clone(),
            done: Arc::new(Mutex::new(false)),
        }
    }

    fn set_head(&self, status: u16) {
        *self.status_code.lock().unwrap() = Some(status);
        *self.status.lock().unwrap() = Some(if (200..400).contains(&status) {
            "ok"
        } else {
            "upstream_error"
        });
    }

    fn set_error(&self, status: &'static str) {
        *self.status.lock().unwrap() = Some(status);
    }

    fn record_once(&self, fallback_status: &'static str) {
        let mut done = self.done.lock().unwrap();
        if *done {
            return;
        }
        *done = true;
        drop(done);

        let status = self.status.lock().unwrap().unwrap_or(fallback_status);
        let status_code = *self.status_code.lock().unwrap();
        let latency_ms = self.started.elapsed().as_millis() as i64;
        let rec = ExecutionRecord {
            request_id: self.request_id.clone(),
            workflow_id: self.workflow_id.clone(),
            slug: self.slug.clone(),
            protocol: self.protocol.clone(),
            status,
            status_code,
            latency_ms: Some(latency_ms),
            bytes_in: self.bytes_in.load(Ordering::Relaxed),
            bytes_out: self.bytes_out.load(Ordering::Relaxed),
            started_at: self.started_at.clone(),
        };
        let db = self.db.clone();
        tokio::spawn(async move {
            db.record_execution(&rec).await;
        });
        debug!(target: "gateflow::exec", request = %self.request_id, latency_ms, "execution recorded");
    }
}

impl Drop for ExecutionRecorder {
    fn drop(&mut self) {
        // Fires on natural completion too; `done` flag dedupes.
        self.record_once("client_disconnect");
    }
}

/// Map the tail frame stream into an axum Body, recording completion.
fn frames_to_body(
    tail: gateflow_engine::FrameStream,
    recorder: ExecutionRecorder,
) -> impl futures::Stream<Item = Result<Bytes, axum::Error>> {
    tail.map(move |frame| match frame {
        Ok(StreamFrame::Raw(b)) => Ok(b),
        Ok(StreamFrame::Event { data, .. }) => Ok(data),
        Ok(StreamFrame::End) | Ok(StreamFrame::Head(_)) => Ok(Bytes::new()),
        Err(e) => {
            recorder.record_once("upstream_error");
            warn!(target: "gateflow::dataplane", error = %e, "stream aborted mid-response");
            Err(axum::Error::new(e))
        }
    })
}

// ---------------------------------------------------------------------------
// Body prefix peek
// ---------------------------------------------------------------------------

/// Read up to `max` bytes of the request body for protocol sniffing, then
/// return those bytes followed by the untouched remainder of the stream.
async fn peek_prefix(body: Body, max: usize, budget: std::time::Duration) -> (Vec<u8>, Body) {
    let mut stream = body.into_data_stream();
    let mut prefix: Vec<u8> = Vec::with_capacity(max.min(4096));
    let mut buffered: Vec<Bytes> = Vec::new();
    let mut error: Option<axum::Error> = None;

    loop {
        let next = tokio::select! {
            _ = tokio::time::sleep(budget) => None,
            n = stream.next() => n,
        };
        let Some(chunk) = next else { break };
        match chunk {
            Ok(b) => {
                if prefix.len() < max {
                    let take = (max - prefix.len()).min(b.len());
                    prefix.extend_from_slice(&b[..take]);
                }
                buffered.push(b);
            }
            Err(e) => {
                error = Some(e);
                break;
            }
        }
        if prefix.len() >= max {
            break;
        }
    }

    let head_stream = futures::stream::iter(buffered.into_iter().map(Ok::<Bytes, axum::Error>));
    let rest = if let Some(e) = error {
        futures::stream::once(async move { Err(e) }).boxed()
    } else {
        stream.boxed()
    };
    let full = head_stream.chain(rest);
    (prefix, Body::from_stream(full))
}

fn resolve_protocol(
    deployed: &Deployed,
    path: &str,
    headers: &HeaderMap,
    prefix: &[u8],
) -> Protocol {
    // Ingress config wins; otherwise sniff.
    if let Some(hint) = &deployed.ingress.protocol_hint {
        return match hint.as_str() {
            "openai" => Protocol::Openai,
            "anthropic" => Protocol::Anthropic,
            "gemini" => Protocol::Gemini,
            _ => Protocol::Raw,
        };
    }
    let hints = DetectHints {
        path: path.to_string(),
        headers: headers.clone(),
        body_prefix: prefix.to_vec(),
    };
    detect(&hints)
}

fn json_error(status: StatusCode, msg: &str) -> Response {
    debug!(target: "gateflow::dataplane", status = status.as_u16(), msg, "returning error");
    (
        status,
        axum::Json(crate::models::ErrorResponse {
            error: msg.to_string(),
        }),
    )
        .into_response()
}

/// `chrono`-free UTC RFC3339 timestamp (matches SQLite's default format).
fn utc_rfc3339_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() as i64;
    let millis = now.subsec_millis() as i64;

    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);

    // Civil-from-days (Howard Hinnant algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if mth <= 2 { y + 1 } else { y };

    format!("{year:04}-{mth:02}-{d:02}T{h:02}:{m:02}:{s:02}.{millis:03}Z")
}
