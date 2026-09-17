//! DAG model, validation, and linearization into an executable chain.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::node::NodeKind;

pub type NodeId = String;

/// A node in the stored workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDef {
    pub id: NodeId,
    pub kind: NodeKind,
    pub name: String,
    #[serde(default)]
    pub config: serde_json::Value,
}

/// A directed edge from `from` to `to`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

/// Serialized workflow graph (stored as `definition` JSON in SQLite).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkflowDag {
    pub nodes: Vec<NodeDef>,
    pub edges: Vec<Edge>,
}

impl WorkflowDag {
    pub fn node(&self, id: &str) -> Option<&NodeDef> {
        self.nodes.iter().find(|n| n.id == id)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DagError {
    #[error("ingress node count must be exactly 1, found {0}")]
    IngressNotUnique(usize),

    #[error("egress node count must be exactly 1, found {0}")]
    EgressNotUnique(usize),

    #[error("unknown node `{0}` referenced by an edge")]
    UnknownNode(String),

    #[error("duplicate edge {from} -> {to}")]
    DuplicateEdge { from: String, to: String },

    #[error("cycle detected in graph; cycle nodes: {0:?}")]
    Cycle(Vec<String>),

    #[error("node `{node}` has {in_degree} incoming edges and {out_degree} outgoing edges; only linear chains are supported in MVP")]
    NotLinear {
        node: String,
        in_degree: usize,
        out_degree: usize,
    },

    #[error("node kind `{kind:?}` is not wired in MVP (only Ingress/Egress)")]
    UnsupportedKind { kind: NodeKind, id: String },

    #[error("missing node config for `{0}`")]
    MissingConfig(String),

    #[error("invalid node config for `{0}`: {1}")]
    InvalidConfig(String, String),
}

/// Kahn's topological sort over the graph's node IDs. Returns nodes in a
/// valid execution order.
pub fn topological_order(dag: &WorkflowDag) -> Result<Vec<NodeId>, DagError> {
    let mut indegree: HashMap<&str, usize> = HashMap::new();
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut seen_edges: HashSet<(&str, &str)> = HashSet::new();

    for node in &dag.nodes {
        indegree.insert(node.id.as_str(), 0);
    }

    for edge in &dag.edges {
        if !indegree.contains_key(edge.from.as_str()) {
            return Err(DagError::UnknownNode(edge.from.clone()));
        }
        if !indegree.contains_key(edge.to.as_str()) {
            return Err(DagError::UnknownNode(edge.to.clone()));
        }
        if !seen_edges.insert((edge.from.as_str(), edge.to.as_str())) {
            return Err(DagError::DuplicateEdge {
                from: edge.from.clone(),
                to: edge.to.clone(),
            });
        }
        *indegree.get_mut(edge.to.as_str()).unwrap() += 1;
        adj.entry(edge.from.as_str()).or_default().push(edge.to.as_str());
    }

    let mut queue: VecDeque<&str> = dag
        .nodes
        .iter()
        .filter(|n| indegree[n.id.as_str()] == 0)
        .map(|n| n.id.as_str())
        .collect();

    let mut order: Vec<NodeId> = Vec::with_capacity(dag.nodes.len());
    while let Some(node) = queue.pop_front() {
        order.push(node.to_string());
        if let Some(nexts) = adj.get(node) {
            for next in nexts {
                let deg = indegree.get_mut(next).unwrap();
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(next);
                }
            }
        }
    }

    if order.len() != dag.nodes.len() {
        let remaining: Vec<String> = dag
            .nodes
            .iter()
            .map(|n| n.id.clone())
            .filter(|id| !order.contains(id))
            .collect();
        return Err(DagError::Cycle(remaining));
    }

    Ok(order)
}

/// Linearity check: every node must have at most one incoming and one
/// outgoing edge (MVP chain). Returns per-node in/out degrees for errors.
fn linearity_check(dag: &WorkflowDag) -> Result<(), DagError> {
    let mut in_deg: HashMap<&str, usize> = HashMap::new();
    let mut out_deg: HashMap<&str, usize> = HashMap::new();
    for n in &dag.nodes {
        in_deg.insert(n.id.as_str(), 0);
        out_deg.insert(n.id.as_str(), 0);
    }
    for e in &dag.edges {
        *out_deg.get_mut(e.from.as_str()).unwrap() += 1;
        *in_deg.get_mut(e.to.as_str()).unwrap() += 1;
    }
    for n in &dag.nodes {
        let id = n.id.as_str();
        if in_deg[id] > 1 || out_deg[id] > 1 {
            return Err(DagError::NotLinear {
                node: id.to_string(),
                in_degree: in_deg[id],
                out_degree: out_deg[id],
            });
        }
    }
    Ok(())
}

/// Validate the DAG and return the ordered list of node builders to instantiante.
///
/// MVP rule set (kept strict, easily relaxed later):
/// - exactly one Ingress, exactly one Egress
/// - only Ingress/Egress kinds allowed
/// - graph is acyclic and linear (a chain)
pub fn validate(dag: &WorkflowDag) -> Result<Vec<NodeDef>, DagError> {
    let ingress = dag.nodes.iter().filter(|n| n.kind == NodeKind::Ingress).count();
    let egress = dag.nodes.iter().filter(|n| n.kind == NodeKind::Egress).count();
    if ingress != 1 {
        return Err(DagError::IngressNotUnique(ingress));
    }
    if egress != 1 {
        return Err(DagError::EgressNotUnique(egress));
    }

    for n in &dag.nodes {
        if !matches!(n.kind, NodeKind::Ingress | NodeKind::Egress) {
            return Err(DagError::UnsupportedKind {
                kind: n.kind,
                id: n.id.clone(),
            });
        }
    }

    let order = topological_order(dag)?;
    linearity_check(dag)?;

    let mut ordered: Vec<NodeDef> = Vec::with_capacity(order.len());
    for id in order {
        if let Some(node) = dag.node(&id) {
            ordered.push(node.clone());
        }
    }
    Ok(ordered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn chain(extra_nodes: Vec<(NodeId, NodeKind)>) -> WorkflowDag {
        let mut nodes = vec![
            NodeDef {
                id: "in".into(),
                kind: NodeKind::Ingress,
                name: "ingress".into(),
                config: json!({}),
            },
            NodeDef {
                id: "out".into(),
                kind: NodeKind::Egress,
                name: "egress".into(),
                config: json!({}),
            },
        ];
        for (id, kind) in extra_nodes {
            nodes.push(NodeDef {
                id: id.clone(),
                kind,
                name: id,
                config: json!({}),
            });
        }
        WorkflowDag {
            nodes,
            edges: vec![Edge {
                from: "in".into(),
                to: "out".into(),
            }],
        }
    }

    #[test]
    fn valid_chain_passes() {
        let dag = chain(vec![]);
        let ordered = validate(&dag).unwrap();
        assert_eq!(ordered.len(), 2);
        assert_eq!(ordered[0].kind, NodeKind::Ingress);
        assert_eq!(ordered[1].kind, NodeKind::Egress);
    }

    #[test]
    fn missing_ingress_fails() {
        let dag = WorkflowDag {
            nodes: vec![NodeDef {
                id: "out".into(),
                kind: NodeKind::Egress,
                name: "e".into(),
                config: json!({}),
            }],
            edges: vec![],
        };
        assert!(matches!(
            validate(&dag),
            Err(DagError::IngressNotUnique(0))
        ));
    }

    #[test]
    fn multiple_egress_fails() {
        let mut dag = chain(vec![]);
        dag.nodes.push(NodeDef {
            id: "out2".into(),
            kind: NodeKind::Egress,
            name: "e2".into(),
            config: json!({}),
        });
        assert!(matches!(
            validate(&dag),
            Err(DagError::EgressNotUnique(2))
        ));
    }

    #[test]
    fn cycle_fails() {
        let dag = WorkflowDag {
            nodes: vec![
                NodeDef {
                    id: "a".into(),
                    kind: NodeKind::Ingress,
                    name: "a".into(),
                    config: json!({}),
                },
                NodeDef {
                    id: "b".into(),
                    kind: NodeKind::Egress,
                    name: "b".into(),
                    config: json!({}),
                },
            ],
            edges: vec![
                Edge { from: "a".into(), to: "b".into() },
                Edge { from: "b".into(), to: "a".into() },
            ],
        };
        assert!(matches!(validate(&dag), Err(DagError::Cycle(_))));
    }

    #[test]
    fn fan_out_fails() {
        // MVP allows only Ingress/Egress kinds, so fan-out also trips the
        // unique-egress rule before linearity. Either rejection is correct.
        let dag = WorkflowDag {
            nodes: vec![
                NodeDef {
                    id: "in".into(),
                    kind: NodeKind::Ingress,
                    name: "i".into(),
                    config: json!({}),
                },
                NodeDef {
                    id: "e1".into(),
                    kind: NodeKind::Egress,
                    name: "e1".into(),
                    config: json!({}),
                },
                NodeDef {
                    id: "e2".into(),
                    kind: NodeKind::Egress,
                    name: "e2".into(),
                    config: json!({}),
                },
            ],
            edges: vec![
                Edge { from: "in".into(), to: "e1".into() },
                Edge { from: "in".into(), to: "e2".into() },
            ],
        };
        assert!(validate(&dag).is_err());
    }

    #[test]
    fn unsupported_kind_fails() {
        let dag = WorkflowDag {
            nodes: vec![
                NodeDef {
                    id: "in".into(),
                    kind: NodeKind::Ingress,
                    name: "i".into(),
                    config: json!({}),
                },
                NodeDef {
                    id: "cv".into(),
                    kind: NodeKind::Convert,
                    name: "cv".into(),
                    config: json!({}),
                },
                NodeDef {
                    id: "out".into(),
                    kind: NodeKind::Egress,
                    name: "o".into(),
                    config: json!({}),
                },
            ],
            edges: vec![
                Edge { from: "in".into(), to: "cv".into() },
                Edge { from: "cv".into(), to: "out".into() },
            ],
        };
        assert!(matches!(
            validate(&dag),
            Err(DagError::UnsupportedKind { .. })
        ));
    }

    #[test]
    fn unknown_node_edge_fails() {
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
            edges: vec![Edge { from: "in".into(), to: "ghost".into() }],
        };
        assert!(matches!(validate(&dag), Err(DagError::UnknownNode(_))));
    }
}