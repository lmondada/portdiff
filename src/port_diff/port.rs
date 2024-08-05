use crate::{
    port::{BoundPort, Site},
    Graph, PortDiff,
};
use derive_more::{From, Into};

use super::IncomingEdgeIndex;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, From, Into)]
pub(super) struct BoundaryIndex(usize);

pub(super) struct BoundaryPort<G: Graph> {
    port: Site<G::Node, G::PortLabel>,
    incoming_edge: IncomingEdgeIndex,
}

pub(super) struct ParentPort<E> {
    incoming_edge: IncomingEdgeIndex,
    port: Port<E>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Port<E> {
    Boundary(BoundaryIndex),
    Internal(BoundPort<E>),
}

pub struct FatPort<G: Graph> {
    pub port: Port<G>,
    pub owner: PortDiff<G>,
}
