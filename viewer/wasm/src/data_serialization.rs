use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Position {
    x: isize,
    y: isize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct InternalNodeData {
    port_diff_id: String,
    n_inputs: usize,
    n_outputs: usize,
    label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ExternalNodeData {
    n_inputs: usize,
    n_outputs: usize,
    label: String,
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
    position: Position,
    data: InternalNodeData,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct BoundaryNode {
    id: String,
    position: Position,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ExternalNode {
    id: String,
    position: Position,
    data: ExternalNodeData,
}

impl Node {
    pub(crate) fn new_internal(x: isize, y: isize) -> Self {
        let id = Uuid::new_v4().to_string();
        Self::Internal(InternalNode {
            id: id.clone(),
            position: Position { x, y },
            data: InternalNodeData {
                port_diff_id: "0".to_string(),
                n_inputs: 1,
                n_outputs: 1,
                label: id,
            },
        })
    }

    pub(crate) fn new_external(x: isize, y: isize) -> Self {
        let id = Uuid::new_v4().to_string();
        Self::External(ExternalNode {
            id: id.clone(),
            position: Position { x, y },
            data: ExternalNodeData {
                n_inputs: 1,
                n_outputs: 1,
                label: id,
            },
        })
    }

    pub(crate) fn new_boundary(x: isize, y: isize) -> Self {
        let id = Uuid::new_v4().to_string();
        Self::Boundary(BoundaryNode {
            id,
            position: Position { x, y },
        })
    }

    pub(crate) fn id(&self) -> &str {
        match self {
            Self::Internal(node) => &node.id,
            Self::External(node) => &node.id,
            Self::Boundary(node) => &node.id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Edge {
    id: String,
    source: String,
    target: String,
}

impl Edge {
    pub(crate) fn new(source: &Node, target: &Node) -> Self {
        let id = Uuid::new_v4().to_string();
        Self {
            id,
            source: source.id().to_string(),
            target: target.id().to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
}

impl Graph {
    pub(crate) fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub(crate) fn new(nodes: Vec<Node>, edges: Vec<Edge>) -> Self {
        Self { nodes, edges }
    }
}

// impl From<&[PortEdge]> for Graph {
//     fn from(value: &[PortEdge]) -> Self {
//         let nodes = value
//             .iter()
//             .flat_map(|edge| {
//                 [
//                     InternalNode::new(edge.left.node.id()),
//                     InternalNode::new(edge.right.node.id()),
//                 ]
//             })
//             .unique()
//             .collect();
//         let edges = value
//             .iter()
//             .enumerate()
//             .map(|(i, edge)| Edge {
//                 id: i.to_string(),
//                 source: edge.left.node.id().to_string(),
//                 target: edge.right.node.id().to_string(),
//             })
//             .collect();
//         Graph { nodes, edges }
//     }
// }
