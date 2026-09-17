//! Concrete node implementations + the node factory the engine uses to
//! compile workflows. Ingress is a pass-through; Egress pumps the request
//! stream into the upstream and re-streams the response with a Head frame.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_stream::try_stream;
use futures::StreamExt;
use gateflow_engine::{
    DagError, ExecCtx, Executable, FrameStream, NodeDef, NodeError, NodeFactory, NodeKind,
    StreamFrame, StreamHead, StreamNode,
};
use http::Method;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

// ---------------------------------------------------------------------------
// Configs (stored in NodeDef.config)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct IngressConfig {
    /// HTTP method the client must use: "ANY" or "GET"/"POST"/...
    pub method: String,
    /// "auto" | "openai" | "anthropic" | "gemini" | "raw"
    pub protocol: String,
    /// Overall execution budget for this workflow.
    pub timeout_ms: Option<u64>,
}

impl Default for IngressConfig {
    fn default() -> Self {
        Self {
            method: "ANY".into(),
            protocol: "auto".into(),
            timeout_ms: Some(120_000),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EgressConfig {
    pub upstream_url: String,
    pub method: String,
    /// Static headers attached to the upstream request (override passthrough).
    pub headers: HashMap<String, String>,
    /// Client request headers forwarded to upstream unless overridden.
    pub passthrough_headers: Vec<String>,
    /// Upstream response headers copied to the gateway response.
    pub forward_response_headers: Vec<String>,
    pub connect_timeout_ms: Option<u64>,
    pub timeout_ms: Option<u64>,
}

impl Default for EgressConfig {
    fn default() -> Self {
        Self {
            upstream_url: String::new(),
            method: "POST".into(),
            headers: HashMap::new(),
            passthrough_headers: vec![
                "authorization".into(),
                "accept".into(),
                "content-type".into(),
            ],
            forward_response_headers: vec![
                "content-type".into(),
                "cache-control".into(),
                "x-request-id".into(),
                "retry-after".into(),
            ],
            connect_timeout_ms: None,
            timeout_ms: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Registry / factory
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Registry {
    pub http: reqwest::Client,
    pub default_connect_timeout: Duration,
    pub default_timeout: Duration,
}

impl Registry {
    pub fn new(config: &crate::config::Config) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(config.default_connect_timeout)
            .http2_adaptive_window(true)
            .pool_idle_timeout(Duration::from_secs(90))
            .build()?;
        Ok(Self {
            http,
            default_connect_timeout: config.default_connect_timeout,
            default_timeout: config.default_timeout,
        })
    }
}

impl NodeFactory for Registry {
    fn build(&self, def: &NodeDef) -> Result<Arc<dyn StreamNode>, DagError> {
        match def.kind {
            NodeKind::Ingress => {
                let cfg: IngressConfig = serde_json::from_value(def.config.clone())
                    .map_err(|e| DagError::InvalidConfig(def.id.clone(), e.to_string()))?;
                Ok(Arc::new(IngressNode { id: def.id.clone(), cfg }))
            }
            NodeKind::Egress => {
                let cfg: EgressConfig = serde_json::from_value(def.config.clone())
                    .map_err(|e| DagError::InvalidConfig(def.id.clone(), e.to_string()))?;
                if cfg.upstream_url.trim().is_empty() {
                    return Err(DagError::InvalidConfig(
                        def.id.clone(),
                        "upstream_url is required".into(),
                    ));
                }
                let url = cfg
                    .upstream_url
                    .parse::<reqwest::Url>()
                    .map_err(|e| DagError::InvalidConfig(def.id.clone(), format!("upstream_url: {e}")))?;
                Ok(Arc::new(EgressNode {
                    id: def.id.clone(),
                    cfg,
                    url,
                    http: self.http.clone(),
                    default_timeout: self.default_timeout,
                }))
            }
            other => Err(DagError::UnsupportedKind {
                kind: other,
                id: def.id.clone(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Ingress node — identity (routing/auth handled by the data-plane)
// ---------------------------------------------------------------------------

pub struct IngressNode {
    pub id: String,
    pub cfg: IngressConfig,
}

impl IngressNode {
    /// Does the request method match this ingress?
    pub fn matches(&self, method: &Method) -> bool {
        self.cfg.method.eq_ignore_ascii_case("ANY")
            || method.as_str().eq_ignore_ascii_case(&self.cfg.method)
    }
}

impl StreamNode for IngressNode {
    fn kind(&self) -> NodeKind {
        NodeKind::Ingress
    }
    fn node_id(&self) -> &str {
        &self.id
    }
    fn run(&self, _ctx: Arc<ExecCtx>, input: FrameStream) -> FrameStream {
        input
    }
}

// ---------------------------------------------------------------------------
// Egress node — stream request body up, stream response body down
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct EgressNode {
    pub id: String,
    pub cfg: EgressConfig,
    url: reqwest::Url,
    http: reqwest::Client,
    default_timeout: Duration,
}

impl StreamNode for EgressNode {
    fn kind(&self) -> NodeKind {
        NodeKind::Egress
    }
    fn node_id(&self) -> &str {
        &self.id
    }

    fn run(&self, ctx: Arc<ExecCtx>, input: FrameStream) -> FrameStream {
        let this = self.clone();
        let ctx2 = ctx.clone();
        Box::pin(try_stream! {
            // Fold upstream URL with a query/path passthrough? MVP: exact URL.
            let url = this.url.clone();

            // --- build request headers: explicit > passthrough ---
            let method = Method::from_bytes(this.cfg.method.as_bytes())
                .map_err(|e| NodeError::node(&this.id, format!("invalid egress method: {e}")))?;
            let mut builder = this.http.request(method, url);

            for (k, v) in &this.cfg.headers {
                builder = builder.header(k, v);
            }
            for name in &this.cfg.passthrough_headers {
                if this.cfg.headers.contains_key(name.as_str()) {
                    continue; // explicit wins
                }
                if let Some(value) = ctx2.request_headers.get(name) {
                    if let Ok(v) = value.to_str() {
                        builder = builder.header(name, v);
                    }
                }
            }
            if !this.cfg.headers.contains_key("accept") && !ctx2.request_headers.contains_key("accept") {
                builder = builder.header("accept", "text/event-stream");
            }
            if let Some(c) = this.cfg.timeout_ms {
                builder = builder.timeout(Duration::from_millis(c.max(this.default_timeout.as_millis() as u64)));
            }

            // --- stream the request body up ---
            let body_frames = gateflow_engine::frames_to_bytes(input);
            builder = builder.body(reqwest::Body::wrap_stream(body_frames));

            let connect_timeout = Duration::from_millis(
                this.cfg.connect_timeout_ms.unwrap_or(this.default_timeout.as_millis() as u64),
            );

            let resp = tokio::time::timeout(connect_timeout, builder.send())
                .await
                .map_err(|_| NodeError::Deadline { deadline: connect_timeout })?
                .map_err(|e| NodeError::Upstream(format!("connect to {}: {e}", this.url)))?;

            let status = resp.status().as_u16();
            let head_headers: Vec<(String, String)> = this
                .cfg
                .forward_response_headers
                .iter()
                .filter_map(|name| {
                    resp.headers()
                        .get(name)
                        .and_then(|v| v.to_str().ok())
                        .map(|v| (name.clone(), v.to_string()))
                })
                .collect();
            debug!(target: "gateflow::egress", workflow = %ctx2.workflow_slug, upstream_status = status, "upstream responded");

            yield StreamFrame::Head(StreamHead {
                status,
                headers: head_headers,
            });

            // --- stream the response body down, respecting deadline + abort ---
            let abort: CancellationToken = ctx2.abort.clone();
            let deadline = ctx2.deadline;
            let started = tokio::time::Instant::now();
            let mut chunks = resp.bytes_stream();

            let result = loop {
                let next = match deadline {
                    Some(dl) => tokio::select! {
                        _ = tokio::time::sleep_until(dl) => break Err(NodeError::Deadline {
                            deadline: dl.saturating_duration_since(started),
                        }),
                        _ = abort.cancelled() => break Ok(()),
                        chunk = chunks.next() => chunk,
                    },
                    None => tokio::select! {
                        _ = abort.cancelled() => break Ok(()),
                        chunk = chunks.next() => chunk,
                    },
                };

                match next {
                    Some(Ok(chunk)) => {
                        ctx2.bytes_out.fetch_add(chunk.len() as u64, std::sync::atomic::Ordering::Relaxed);
                        yield StreamFrame::Raw(chunk);
                    }
                    Some(Err(e)) => {
                        warn!(target: "gateflow::egress", error = %e, "upstream stream error");
                        break Err(NodeError::Upstream(format!("stream from {}: {e}", this.url)));
                    }
                    None => break Ok(()),
                }
            };

            // Ends the stream with an error item when the loop broke with Err.
            if let Err(e) = result {
                Err::<(), NodeError>(e)?;
            }
        })
    }
}

// Keep `Executable` generic usage stable for future parallel execution plans.
pub type _Compiled = Executable;