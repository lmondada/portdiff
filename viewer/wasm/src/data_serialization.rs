use portdiff::UniqueVertex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Port, PortEdge, PortLabel};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct InternalNodeData {
    port_diff_id: Uuid,
    label: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ExternalNodeData {
    label: String,
}

impl From<InternalNodeData> for ExternalNodeData {
    fn from(data: InternalNodeData) -> Self {
        let InternalNodeData { label, .. } = data;
        ExternalNodeData { label }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum Node {
    Internal(InternalNode),
    Boundary(BoundaryNode),
    External(ExternalNode),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct InternalNode {
    id: Uuid,
    data: InternalNodeData,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct BoundaryNode {
    id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ExternalNode {
    id: Uuid,
    data: ExternalNodeData,
}

impl Node {
    pub(crate) fn new_internal(id: Uuid, port_diff_id: Uuid) -> Self {
        Self::Internal(InternalNode {
            id,
            data: InternalNodeData {
                port_diff_id,
                label: id.to_string(),
            },
        })
    }

    pub(crate) fn new_external(id: Uuid) -> Self {
        Self::External(ExternalNode {
            id,
            data: ExternalNodeData {
                label: id.to_string(),
            },
        })
    }

    pub(crate) fn new_boundary() -> Self {
        let id = Uuid::new_v4();
        Self::Boundary(BoundaryNode { id })
    }

    pub(crate) fn id(&self) -> Uuid {
        match self {
            Self::Internal(node) => node.id,
            Self::External(node) => node.id,
            Self::Boundary(node) => node.id,
        }
    }

    pub(crate) fn is_external(&self) -> bool {
        matches!(self, Self::External(_))
    }

    pub(crate) fn is_internal(&self) -> bool {
        matches!(self, Self::Internal(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Edge {
    pub(crate) id: Uuid,
    pub(crate) source: Uuid,
    pub(crate) source_handle: usize,
    pub(crate) target: Uuid,
    pub(crate) target_handle: usize,
}

impl Edge {
    pub(crate) fn new(
        source: Uuid,
        source_handle: usize,
        target: Uuid,
        target_handle: usize,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            source,
            target,
            source_handle,
            target_handle,
        }
    }
}

impl From<&PortEdge> for Edge {
    fn from(edge: &PortEdge) -> Self {
        Edge::new(
            edge.left.node.id(),
            edge.left.port.index(),
            edge.right.node.id(),
            edge.right.port.index(),
        )
    }
}

impl From<&Edge> for PortEdge {
    fn from(edge: &Edge) -> Self {
        PortEdge {
            left: Port {
                node: UniqueVertex::from_id(edge.source),
                port: PortLabel::Out(edge.source_handle),
            },
            right: Port {
                node: UniqueVertex::from_id(edge.target),
                port: PortLabel::In(edge.target_handle),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Graph {
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
}

impl Graph {
    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub(crate) fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Self {
        Self { nodes, edges }
    }
}
