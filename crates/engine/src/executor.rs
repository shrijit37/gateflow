//! Compilation of a validated DAG into a runnable chain.

use std::sync::Arc;

use crate::dag::{validate, DagError, NodeDef, WorkflowDag};
use crate::frame::FrameStream;
use crate::node::ExecCtx;
use crate::node::StreamNode;

/// Builds concrete node instances from serialized definitions.
///
/// The engine is node-agnostic; the host application (the Gateflow server)
/// implements this with its node registry.
pub trait NodeFactory: Send + Sync {
    fn build(&self, def: &NodeDef) -> Result<Arc<dyn StreamNode>, DagError>;
}

/// A compiled, immutable workflow ready to execute. Cheap to clone (`Arc`).
#[derive(Clone)]
pub struct Executable {
    pub workflow_id: String,
    pub workflow_slug: String,
    /// Nodes in execution (topological) order.
    pub chain: Arc<Vec<Arc<dyn StreamNode>>>,
}

impl Executable {
    /// Thread the input stream through every node in order.
    ///
    /// Each node receives a [`FrameStream`] and returns one; nodes that need
    /// concurrency spawn their own tasks and connect via bounded channels.
    pub fn execute(&self, ctx: Arc<ExecCtx>, input: FrameStream) -> FrameStream {
        let mut stream = input;
        for node in self.chain.iter() {
            stream = node.run(ctx.clone(), stream);
        }
        stream
    }
}

/// Validate a raw DAG and compile it via the provided node factory.
pub fn compile(
    workflow_id: impl Into<String>,
    workflow_slug: impl Into<String>,
    dag: &WorkflowDag,
    factory: &dyn NodeFactory,
) -> Result<Executable, DagError> {
    let ordered = validate(dag)?;
    let mut chain: Vec<Arc<dyn StreamNode>> = Vec::with_capacity(ordered.len());
    for def in ordered {
        chain.push(factory.build(&def)?);
    }
    Ok(Executable {
        workflow_id: workflow_id.into(),
        workflow_slug: workflow_slug.into(),
        chain: Arc::new(chain),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{Edge, NodeDef};
    use crate::frame::{bytes_to_frames, StreamFrame};
    use crate::node::{NodeKind, NodeError};
    use futures::StreamExt;
    use serde_json::json;

    #[derive(Clone)]
    struct PassNode {
        id: String,
        kind: NodeKind,
    }

    impl StreamNode for PassNode {
        fn kind(&self) -> NodeKind {
            self.kind
        }
        fn node_id(&self) -> &str {
            &self.id
        }
        fn run(&self, _ctx: Arc<ExecCtx>, input: FrameStream) -> FrameStream {
            input
        }
    }

    struct TestFactory;
    impl NodeFactory for TestFactory {
        fn build(&self, def: &NodeDef) -> Result<Arc<dyn StreamNode>, DagError> {
            Ok(Arc::new(PassNode {
                id: def.id.clone(),
                kind: def.kind,
            }))
        }
    }

    #[tokio::test]
    async fn executes_passthrough_chain() {
        let dag = WorkflowDag {
            nodes: vec![
                NodeDef {
                    id: "in".into(),
                    kind: NodeKind::Ingress,
                    name: "i".into(),
                    config: json!({}),
                },
                NodeDef {
                    id: "out".into(),
                    kind: NodeKind::Egress,
                    name: "o".into(),
                    config: json!({}),
                },
            ],
            edges: vec![Edge {
                from: "in".into(),
                to: "out".into(),
            }],
        };

        let exe = compile("wf1", "slug1", &dag, &TestFactory).unwrap();
        assert_eq!(exe.chain.len(), 2);

        let ctx = ExecCtx::new(
            "req1",
            "wf1",
            "slug1",
            crate::frame::Protocol::Raw,
            http::HeaderMap::new(),
            tokio_util::sync::CancellationToken::new(),
        );

        let input = bytes_to_frames(
            vec![
                bytes::Bytes::from_static(b"hello "),
                bytes::Bytes::from_static(b"world"),
            ]
            .into_iter(),
        );

        let mut out = exe.execute(ctx, input);
        let mut collected = Vec::new();
        while let Some(frame) = out.next().await {
            match frame.unwrap() {
                StreamFrame::Raw(b) => collected.push(b),
                other => panic!("unexpected frame: {other:?}"),
            }
        }
        assert_eq!(collected, vec![
            bytes::Bytes::from_static(b"hello "),
            bytes::Bytes::from_static(b"world"),
        ]);
    }

    #[tokio::test]
    async fn error_frame_propagates() {
        let dag = WorkflowDag {
            nodes: vec![
                NodeDef {
                    id: "in".into(),
                    kind: NodeKind::Ingress,
                    name: "i".into(),
                    config: json!({}),
                },
                NodeDef {
                    id: "out".into(),
                    kind: NodeKind::Egress,
                    name: "o".into(),
                    config: json!({}),
                },
            ],
            edges: vec![Edge {
                from: "in".into(),
                to: "out".into(),
            }],
        };
        let _exe = compile("wf2", "slug2", &dag, &TestFactory).unwrap();

        struct Boom;
        impl StreamNode for Boom {
            fn kind(&self) -> NodeKind {
                NodeKind::Ingress
            }
            fn node_id(&self) -> &str {
                "boom"
            }
            fn run(&self, _ctx: Arc<ExecCtx>, _input: FrameStream) -> FrameStream {
                Box::pin(futures::stream::iter(vec![Err(NodeError::Node {
                    node: "boom".into(),
                    msg: "kaboom".into(),
                })]))
            }
        }

        let ctx = ExecCtx::new(
            "req2",
            "wf2",
            "slug2",
            crate::frame::Protocol::Raw,
            http::HeaderMap::new(),
            tokio_util::sync::CancellationToken::new(),
        );
        let bad_exe = Executable {
            workflow_id: "wf2".into(),
            workflow_slug: "slug2".into(),
            chain: Arc::new(vec![Arc::new(Boom)]),
        };
        let mut out = bad_exe.execute(ctx, bytes_to_frames(vec![].into_iter()));
        let frame = out.next().await.unwrap();
        assert!(matches!(frame, Err(NodeError::Node { .. })));
    }
}