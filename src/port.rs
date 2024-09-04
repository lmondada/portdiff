//! Data types for ports

use std::fmt::Debug;
use std::hash::Hash;

use derive_more::{From, Into};
use derive_where::derive_where;
use serde::{Deserialize, Serialize};

use crate::{port_diff::Owned, Graph};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EdgeEnd {
    Left,
    Right,
}

impl EdgeEnd {
    pub fn opposite(&self) -> Self {
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
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
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
    pub fn site(&self) -> Option<Site<G::Node, G::PortLabel>> {
        match self.data {
            Port::Boundary(boundary) => self.owner.boundary_site(boundary).clone().try_into().ok(),
            Port::Bound(port) => Some(self.owner.graph().get_port_site(port)),
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
}

// impl<G: Graph> From<Site<G::Node, G::PortLabel>> for BoundarySite<G> {
//     fn from(value: Site<G::Node, G::PortLabel>) -> Self {
//         let site = Site {
//             node: value.node,
//             port: value.port,
//         };
//         Self(site)
//     }
// }

impl<G: Graph> TryFrom<BoundarySite<G>> for Site<G::Node, G::PortLabel> {
    type Error = BoundarySite<G>;

    fn try_from(value: BoundarySite<G>) -> Result<Self, Self::Error> {
        value.try_into_site()
    }
}

/// A site of a boundary port.
///
/// Either a site of the graph or a site on an "imaginary" wire. As many
/// such wires can be created as needed. For any wire ID, there may be at
/// most one site for each end. Wires should be assigned increasing indices
/// starting from 0.
#[derive(Serialize, Deserialize, From)]
#[derive_where(PartialEq, Eq, PartialOrd, Ord, Clone; G: Graph)]
#[derive_where(Debug; G: Graph, G::Node: Debug, G::PortLabel: Debug)]
#[derive_where(Hash; G: Graph, G::Node: Hash, G::PortLabel: Hash)]
#[serde(bound(
    serialize = "G::Node: Serialize, G::PortLabel: Serialize",
    deserialize = "G::Node: Deserialize<'de>, G::PortLabel: Deserialize<'de>"
))]
pub enum BoundarySite<G: Graph> {
    Site(Site<G::Node, G::PortLabel>),
    Wire { id: usize, end: EdgeEnd },
}

impl<G: Graph> BoundarySite<G> {
    pub fn try_as_site_ref(&self) -> Option<&Site<G::Node, G::PortLabel>> {
        match self {
            Self::Site(site) => Some(site),
            Self::Wire { .. } => None,
        }
    }

    pub fn try_into_site(self) -> Result<Site<G::Node, G::PortLabel>, Self> {
        match self {
            Self::Site(site) => Ok(site),
            Self::Wire { .. } => Err(self),
        }
    }

    pub fn unwrap_site(self) -> Site<G::Node, G::PortLabel> {
        self.try_into_site().ok().unwrap()
    }
}
