//! Data types for ports

use derive_more::{From, Into};
use derive_where::derive_where;
use serde::{Deserialize, Serialize};

use crate::{port_diff::Owned, Graph};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EdgeEnd {
    Left,
    Right,
}

impl EdgeEnd {
    fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// Site: where ports can be connected.
///
/// Uniquely given by a node and a port label. There may be 0, 1 or multiple
/// ports at the same site.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Site<N, P> {
    /// The node
    pub node: N,
    /// The port label
    pub port: P,
}

impl<N, P> Site<N, P> {
    pub fn map_node(self, f: impl FnOnce(N) -> N) -> Site<N, P> {
        Site {
            node: f(self.node),
            port: self.port,
        }
    }

    pub fn filter_map_node(self, f: impl FnOnce(N) -> Option<N>) -> Option<Site<N, P>> {
        Some(Site {
            node: f(self.node)?,
            port: self.port,
        })
    }
}

/// A boundary port, given by the index of the port in the boundary.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct BoundaryIndex(usize);

/// A port in the graph, either connected to an edge or marking a subgraph boundary.
///
/// The port belongs to a site. There may be 0 or 1 edge connected to a port.
#[derive(Debug, From, Serialize, Deserialize)]
#[serde(bound(
    serialize = "G::Edge: Serialize",
    deserialize = "G::Edge: Deserialize<'de>"
))]
#[derive_where(PartialEq; G: Graph)]
#[derive_where(Eq; G: Graph)]
#[derive_where(PartialOrd; G: Graph)]
#[derive_where(Ord; G: Graph)]
pub enum Port<G: Graph> {
    /// The i-th boundary port of the graph.
    Boundary(BoundaryIndex),
    /// A port connected to an edge.
    Bound(BoundPort<G::Edge>),
}

/// A port that is connected to an edge.
///
/// This is given by a an edge and an edge end. This always determines the
/// port uniquely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BoundPort<E> {
    /// The edge
    pub edge: E,
    /// Whether it is the left or right end of the edge.
    pub end: EdgeEnd,
}

impl<G: Graph> Clone for Port<G> {
    fn clone(&self) -> Self {
        match self {
            Self::Boundary(i) => Self::Boundary(*i),
            Self::Bound(port) => Self::Bound(port.clone()),
        }
    }
}

impl<G: Graph> Copy for Port<G> where G::Edge: Copy {}

impl<G: Graph> Owned<Port<G>, G> {
    pub fn site(&self) -> Site<G::Node, G::PortLabel> {
        match self.data {
            Port::Boundary(boundary) => self.owner.boundary_site(boundary).clone(),
            Port::Bound(port) => self.owner.graph().get_port_site(port),
        }
    }
}

// #[derive(Debug, PartialEq, Eq)]
// pub(crate) struct ParentPort<G: Graph> {
//     pub(crate) parent: PortDiff<G>,
//     pub(crate) port: BoundPort<G::Edge>,
// }

// #[derive(Clone, Debug, PartialEq, Eq)]
// pub(crate) struct ChildPort<G: Graph> {
//     pub(crate) child: WeakPortDiff<G>,
//     pub(crate) port: UnboundPort<G::Node, G::PortLabel>,
// }

impl<E: Copy> BoundPort<E> {
    pub fn opposite(&self) -> Self {
        Self {
            edge: self.edge,
            end: self.end.opposite(),
        }
    }

    // pub(crate) fn to_parent_port<G: Graph<Edge = E>>(&self, owner: PortDiff<G>) -> ParentPort<G> {
    //     ParentPort {
    //         parent: owner,
    //         port: *self,
    //     }
    // }
}

// impl<G: Graph> From<ParentPort<G>> for Port<G> {
//     fn from(port: ParentPort<G>) -> Self {
//         Self::Bound {
//             port: port.port,
//             owner: port.parent,
//         }
//     }
// }

// impl<G: Graph> Clone for ParentPort<G> {
//     fn clone(&self) -> Self {
//         ParentPort {
//             parent: self.parent.clone(),
//             port: self.port.clone(),
//         }
//     }
// }

// impl<G: Graph> ParentPort<G> {
//     pub fn opposite(&self) -> Self {
//         ParentPort {
//             parent: self.parent.clone(),
//             port: self.port.opposite(),
//         }
//     }

//     pub fn children(&self) -> Ref<[ChildPort<G>]> {
//         self.parent.children(self.port)
//     }
// }

// impl<G: Graph> ChildPort<G> {
//     pub fn is_upgradable(&self) -> bool {
//         self.child.is_upgradable()
//     }

//     pub fn upgrade(&self) -> Option<Port<G>> {
//         Some(Port::Unbound {
//             port: self.port.clone(),
//             owner: self.child.upgrade()?,
//         })
//     }
// }

// impl<G: Graph> Port<G> {
//     pub fn owner(&self) -> &PortDiff<G> {
//         match self {
//             Self::Unbound { owner, .. } => owner,
//             Self::Bound { owner, .. } => owner,
//         }
//     }

//     pub fn node(&self) -> G::Node {
//         match self {
//             Self::Unbound { port, .. } => port.node,
//             Self::Bound { port, owner } => owner.graph().to_unbound(*port).node,
//         }
//     }
// }
