//! Node trait, execution context, and error type.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use http::HeaderMap;
use tokio_util::sync::CancellationToken;

use crate::frame::{FrameStream, Protocol};

/// What a node does. Extensible — future kinds plug in without engine changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Ingress,
    Egress,
    Convert,
    Tool,
    Skill,
    Guardrail,
    Cache,
}

impl NodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            NodeKind::Ingress => "ingress",
            NodeKind::Egress => "egress",
            NodeKind::Convert => "convert",
            NodeKind::Tool => "tool",
            NodeKind::Skill => "skill",
            NodeKind::Guardrail => "guardrail",
            NodeKind::Cache => "cache",
        }
    }
}

/// Execution-scoped errors. Converted to HTTP responses by the data-plane.
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("node \"{node}\" failed: {msg}")]
    Node { node: String, msg: String },

    #[error("upstream request failed: {0}")]
    Upstream(String),

    #[error("upstream returned HTTP {status}")]
    UpstreamStatus { status: u16 },

    #[error("stream read failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("deadline exceeded ({deadline:?} budget)")]
    Deadline { deadline: std::time::Duration },

    #[error("execution cancelled")]
    Cancelled,

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl NodeError {
    pub fn node(name: impl Into<String>, msg: impl Into<String>) -> Self {
        NodeError::Node {
            node: name.into(),
            msg: msg.into(),
        }
    }
}

/// Immutable execution metadata shared by every node in a run.
#[derive(Debug, Clone)]
pub struct ExecCtx {
    pub request_id: String,
    pub workflow_id: String,
    pub workflow_slug: String,
    pub protocol: Protocol,
    pub deadline: Option<tokio::time::Instant>,
    pub request_headers: HeaderMap,
    pub abort: CancellationToken,
    pub started_at: tokio::time::Instant,
    /// Node kind of the *terminal* node, so only it emits a head frame.
    pub terminal_kind: NodeKind,
    /// Running byte counters (in/out), shared across the pipeline.
    pub bytes_in: Arc<AtomicU64>,
    pub bytes_out: Arc<AtomicU64>,
}

impl ExecCtx {
    pub fn new(
        request_id: impl Into<String>,
        workflow_id: impl Into<String>,
        workflow_slug: impl Into<String>,
        protocol: Protocol,
        request_headers: HeaderMap,
        abort: CancellationToken,
    ) -> Arc<Self> {
        Arc::new(ExecCtx {
            request_id: request_id.into(),
            workflow_id: workflow_id.into(),
            workflow_slug: workflow_slug.into(),
            protocol,
            deadline: None,
            request_headers,
            abort,
            started_at: tokio::time::Instant::now(),
            terminal_kind: NodeKind::Egress,
            bytes_in: Arc::new(AtomicU64::new(0)),
            bytes_out: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Set or clear the execution deadline. `ExecCtx` is immutable otherwise.
    pub fn with_deadline(mut self: Arc<Self>, deadline: Option<tokio::time::Instant>) -> Arc<Self> {
        if let Some(dl) = deadline {
            Arc::get_mut(&mut self)
                .expect("ctx must not be shared yet")
                .deadline = Some(dl);
        }
        self
    }
    pub fn load(atomic: &AtomicU64) -> u64 {
        atomic.load(Ordering::Relaxed)
    }
}

/// A node transforms an input frame stream into an output frame stream.
///
/// Nodes are cheap to clone (wrapped in `Arc`) and are invoked once per
/// execution. Implementations spawn their own Tokio tasks when they need
/// concurrency; the bounded channel joining the caller's output to this
/// node's input provides backpressure.
pub trait StreamNode: Send + Sync {
    fn kind(&self) -> NodeKind;
    fn run(&self, ctx: Arc<ExecCtx>, input: FrameStream) -> FrameStream;
    fn node_id(&self) -> &str;
}