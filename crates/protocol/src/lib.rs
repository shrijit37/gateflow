//! Protocol detection and SSE framing helpers for the Gateflow gateway.
//!
//! Detection is header-first, payload-sniff second, `Raw` last — and never
//! runs in the passthrough hot path (the data-plane tags once per request,
//! using only headers and a small body prefix peek).

pub mod detect;
pub mod sse;

pub use detect::{detect, DetectHints};
pub use sse::{SseEvent, SseParser};