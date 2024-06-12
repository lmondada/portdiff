mod edges;
pub mod port_diff;
mod unique_vertex;

use std::fmt::Debug;

pub use edges::BoundaryEdge;
pub use port_diff::PortDiff;
pub use unique_vertex::UniqueVertex;

/// A connection point for an edge
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Port<V, P> {
    /// The node
    pub node: V,
    /// The port label
    pub port: P,
}

/// An edge between two ports.
///
/// Note that `source` and `target` do not imply the edge orientation, but its
/// traversal direction (edge orientation should be captured in port labels `P`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortEdge<V, P> {
    /// The source port
    pub left: Port<V, P>,
    /// The target port
    pub right: Port<V, P>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EdgeEndType {
    Left,
    Right,
}

impl EdgeEndType {
    fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}
