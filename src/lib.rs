mod edges;
pub mod port_diff;

pub use port_diff::PortDiff;

/// A connection point for an edge
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Port<V, P> {
    node: V,
    port: P,
}

/// An edge between two ports.
///
/// Note that `source` and `target` do not imply the edge orientation, but its
/// traversal direction (edge orientation should be captured in port labels `P`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortEdge<V, P> {
    /// The source port
    left: Port<V, P>,
    /// The target port
    right: Port<V, P>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
