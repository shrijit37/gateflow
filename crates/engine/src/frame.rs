//! Stream frame IR — the unit of data moving through a workflow.
//!
//! Hot-path nodes exchange [`StreamFrame::Raw`] (`bytes::Bytes`) with zero
//! copying and no parsing. `Event` / `Head` variants exist now so phase-2
//! converter / cache / guardrail nodes can be added without changing the
//! engine. A stream's termination is signaled by the stream ending (`None`),
//! but nodes may emit an explicit [`StreamFrame::End`] to flush buffered
//! state early.

use bytes::Bytes;

use futures::StreamExt as _;

use crate::node::NodeError;

/// Wire protocol carried by a workflow execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Openai,
    Anthropic,
    Gemini,
    Raw,
}

impl Protocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Protocol::Openai => "openai",
            Protocol::Anthropic => "anthropic",
            Protocol::Gemini => "gemini",
            Protocol::Raw => "raw",
        }
    }
}

/// A single unit flowing through the pipeline.
#[derive(Debug, Clone)]
pub enum StreamFrame {
    /// Passthrough bytes — the overwhelmingly common frame. Zero-copy.
    Raw(Bytes),
    /// A semantic event (parsed or synthesized), e.g. an SSE event.
    Event { event: Option<String>, data: Bytes },
    /// Explicit end-of-stream sentinel emitted by a node.
    End,
    /// Emitted only by the *terminal* node as its first frame; carries the
    /// real upstream status + response headers so the data-plane can set the
    /// HTTP response head before streaming bytes. Never used mid-stream.
    Head(StreamHead),
}

impl StreamFrame {
    pub fn raw<B: Into<Bytes>>(b: B) -> Self {
        StreamFrame::Raw(b.into())
    }
}

/// Boxed, pinned stream of frames produced/consumed by every node.
pub type FrameStream =
    std::pin::Pin<Box<dyn futures::Stream<Item = Result<StreamFrame, NodeError>> + Send>>;

/// Convenience: adapt an `Iterator<Item = Bytes>` into a [`FrameStream`].
pub fn bytes_to_frames<I>(iter: I) -> FrameStream
where
    I: Iterator<Item = Bytes> + Send + 'static,
{
    Box::pin(futures::stream::iter(iter.map(|b| Ok(StreamFrame::raw(b)))))
}

/// Convenience: strip frames down to their raw byte payloads (terminates on End).
pub fn frames_to_bytes<S>(frames: S) -> impl futures::Stream<Item = Result<Bytes, NodeError>>
where
    S: futures::Stream<Item = Result<StreamFrame, NodeError>> + Send + 'static,
{
    frames.filter_map(|f| async move {
        match f {
            Ok(StreamFrame::Raw(b)) => Some(Ok(b)),
            Ok(StreamFrame::Event { data, .. }) => Some(Ok(data)),
            Ok(StreamFrame::End) | Ok(StreamFrame::Head(_)) => None,
            Err(e) => Some(Err(e)),
        }
    })
}

/// Metadata carried by the *first* frame of a terminal node's output stream.
///
/// Only the terminal node (egress today; future: cache-hit responders) may
/// emit this. The data-plane reads it before writing any bytes so it can set
/// the real upstream status code and response headers, then streams the rest.
#[derive(Debug, Clone, Default)]
pub struct StreamHead {
    pub status: u16,
    pub headers: Vec<(String, String)>,
}
