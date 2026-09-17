//! Gateflow core engine: a versionless stream-transforming DAG runtime.
//!
//! The engine is intentionally protocol-agnostic. It moves [`StreamFrame`]s:
//! mostly zero-copy raw bytes (passthrough hot path), with richer `Event` and
//! `Head` variants reserved for future converter / cache / guardrail nodes.
//!
//! A workflow is a [`WorkflowDag`] of [`StreamNode`]s. [`compile`] validates and
//! linearizes the DAG into an ordered chain; [`Executable::execute`] threads a
//! [`FrameStream`] through every node, each running as its own Tokio task.

pub mod dag;
pub mod executor;
pub mod frame;
pub mod node;

pub use dag::{DagError, Edge, NodeDef, WorkflowDag};
pub use executor::{compile, Executable, NodeFactory};
pub use frame::{bytes_to_frames, frames_to_bytes, FrameStream, Protocol, StreamFrame, StreamHead};
pub use node::{ExecCtx, NodeError, NodeKind, StreamNode};