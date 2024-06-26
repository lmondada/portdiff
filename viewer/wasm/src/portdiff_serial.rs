use portdiff::DetVertex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{port_diff_id::PortDiffId, Port, PortEdge, PortLabel};

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct InternalNodeData {
    port_diff_id: String,
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
    id: String,
    data: InternalNodeData,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct BoundaryNode {
    id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ExternalNode {
    id: String,
    data: ExternalNodeData,
}

impl Node {
    pub(crate) fn new_internal(id: String, port_diff_id: PortDiffId) -> Self {
        let label = id.clone();
        Self::Internal(InternalNode {
            id,
            data: InternalNodeData {
                port_diff_id: port_diff_id.0,
                label,
            },
        })
    }

    pub(crate) fn new_external(id: String) -> Self {
        let label = id.clone();
        Self::External(ExternalNode {
            id,
            data: ExternalNodeData { label },
        })
    }

    pub(crate) fn new_boundary(id: String) -> Self {
        Self::Boundary(BoundaryNode { id })
    }

    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Internal(node) => &node.id,
            Self::External(node) => &node.id,
            Self::Boundary(node) => &node.id,
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
    pub(crate) source: String,
    pub(crate) source_handle: usize,
    pub(crate) target: String,
    pub(crate) target_handle: usize,
    pub(crate) style: Option<String>,
}

impl Edge {
    pub(crate) fn new(
        source: String,
        source_handle: usize,
        target: String,
        target_handle: usize,
        style: Option<String>,
    ) -> Self {
        let id = Uuid::new_v4();
        Self {
            id,
            source,
            target,
            source_handle,
            target_handle,
            style,
        }
    }

    pub(crate) fn from_ports(p1: &Port, p2: &Port, style: Option<String>) -> Self {
        let (source, target) = match (p1.port, p2.port) {
            (PortLabel::Out(_), PortLabel::In(_)) => (p1, p2),
            (PortLabel::In(_), PortLabel::Out(_)) => (p2, p1),
            _ => panic!("invalid port edge"),
        };
        Edge::new(
            source.node.id().to_string(),
            source.port.index(),
            target.node.id().to_string(),
            target.port.index(),
            style,
        )
    }

    pub(crate) fn from_boundary(port: &Port, boundary: &Node, style: Option<String>) -> Self {
        let boundary_label = match port.port {
            PortLabel::Out(_) => PortLabel::In(0),
            PortLabel::In(_) => PortLabel::Out(0),
        };
        let boundary_port = Port {
            node: DetVertex(boundary.id().to_string()),
            port: boundary_label,
        };
        Edge::from_ports(port, &boundary_port, style)
    }

    pub(crate) fn from_nodes(node1: &Node, node2: &Node, style: Option<String>) -> Self {
        Edge::new(node1.id().to_string(), 0, node2.id().to_string(), 0, style)
    }
}

impl From<&PortEdge> for Edge {
    fn from(edge: &PortEdge) -> Self {
        Edge::from_ports(&edge.left, &edge.right, None)
    }
}

impl From<&Edge> for PortEdge {
    fn from(edge: &Edge) -> Self {
        PortEdge {
            left: Port {
                node: DetVertex(edge.source.clone()),
                port: PortLabel::Out(edge.source_handle),
            },
            right: Port {
                node: DetVertex(edge.target.clone()),
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
}
