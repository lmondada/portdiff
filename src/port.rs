//! Data types for ports

use std::cell::Ref;

use delegate::delegate;

use crate::{port_diff::WeakPortDiff, Graph, PortDiff};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PortSide {
    Left,
    Right,
}

impl PortSide {
    fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

/// A port without connected edge.
///
/// This is given by a node and a port label and may not be unique if the port
/// label is not unique.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnboundPort<N, P> {
    /// The node
    pub node: N,
    /// The port label
    pub port: P,
}

/// A port that is connected to an edge.
///
/// This is given by a an edge and a port side. This always determines the
/// port uniquely.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundPort<E> {
    /// The edge
    pub edge: E,
    /// The port side
    pub port: PortSide,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Port<G: Graph> {
    Unbound {
        port: UnboundPort<G::Node, G::PortLabel>,
        owner: PortDiff<G>,
    },
    Bound {
        port: BoundPort<G::Edge>,
        owner: PortDiff<G>,
    },
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ParentPort<G: Graph> {
    pub(crate) parent: PortDiff<G>,
    pub(crate) port: BoundPort<G::Edge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChildPort<G: Graph> {
    pub(crate) child: WeakPortDiff<G>,
    pub(crate) port: UnboundPort<G::Node, G::PortLabel>,
}

impl<E: Copy> BoundPort<E> {
    pub fn opposite(&self) -> Self {
        Self {
            edge: self.edge,
            port: self.port.opposite(),
        }
    }

    pub(crate) fn to_parent_port<G: Graph<Edge = E>>(&self, owner: PortDiff<G>) -> ParentPort<G> {
        ParentPort {
            parent: owner,
            port: *self,
        }
    }
}

impl<N: Copy, P: Clone> UnboundPort<N, P> {
    pub fn to_port<G: Graph<Node = N, PortLabel = P>>(&self, owner: PortDiff<G>) -> Port<G> {
        Port::Unbound {
            port: self.clone(),
            owner,
        }
    }
}

impl<G: Graph> From<ParentPort<G>> for Port<G> {
    fn from(port: ParentPort<G>) -> Self {
        Self::Bound {
            port: port.port,
            owner: port.parent,
        }
    }
}

impl<G: Graph> Clone for ParentPort<G> {
    fn clone(&self) -> Self {
        ParentPort {
            parent: self.parent.clone(),
            port: self.port.clone(),
        }
    }
}

impl<G: Graph> ParentPort<G> {
    pub fn opposite(&self) -> Self {
        ParentPort {
            parent: self.parent.clone(),
            port: self.port.opposite(),
        }
    }

    pub fn children(&self) -> Ref<[ChildPort<G>]> {
        self.parent.children(self.port)
    }
}

impl<G: Graph> ChildPort<G> {
    pub fn is_upgradable(&self) -> bool {
        self.child.is_upgradable()
    }

    pub fn upgrade(&self) -> Option<Port<G>> {
        Some(Port::Unbound {
            port: self.port.clone(),
            owner: self.child.upgrade()?,
        })
    }
}

impl<G: Graph> Port<G> {
    pub fn owner(&self) -> &PortDiff<G> {
        match self {
            Self::Unbound { owner, .. } => owner,
            Self::Bound { owner, .. } => owner,
        }
    }

    pub fn node(&self) -> G::Node {
        match self {
            Self::Unbound { port, .. } => port.node,
            Self::Bound { port, owner } => owner.graph().to_unbound(*port).node,
        }
    }
}
