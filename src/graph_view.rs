use std::borrow::Borrow;

use derive_more::{From, Into};
use derive_where::derive_where;
use serde::{Deserialize, Serialize};

use crate::{
    port_diff::{EdgeData, PortDiffData},
    Graph, PortDiff,
};

/// A view into a graph that only shows a subset of the nodes.
#[derive(From, Into, Serialize, Deserialize)]
#[derive_where(Clone, Default; G: Graph)]
#[serde(bound(
    serialize = "G: Serialize, G::Node: Serialize, G::PortLabel: Serialize, G::Edge: Serialize",
    deserialize = "G: Deserialize<'de>, G::Node: Deserialize<'de>, G::PortLabel: Deserialize<'de>, G::Edge: Deserialize<'de>"
))]
pub struct GraphView<G: Graph>(relrc::GraphView<PortDiffData<G>, EdgeData<G>>);

/// A handle to a node in a graph view.
#[derive(From, Into)]
#[derive_where(Clone, Copy, PartialEq, Eq, PartialOrd, Ord; G: Graph)]
pub struct NodeId<G: Graph>(relrc::NodeId<PortDiffData<G>, EdgeData<G>>);

impl<'a, G: Graph> From<&'a PortDiff<G>> for NodeId<G> {
    fn from(value: &'a PortDiff<G>) -> Self {
        let node_id: relrc::NodeId<_, _> = (&value.data).into();
        node_id.into()
    }
}

impl<G: Graph> GraphView<G> {
    pub fn all_nodes(&self) -> impl Iterator<Item = NodeId<G>> + '_ {
        self.0.all_nodes().iter().copied().map(|n| n.into())
    }

    pub fn from_sinks(sinks: impl IntoIterator<Item = PortDiff<G>>) -> Self {
        Self(relrc::GraphView::from_sinks(
            sinks.into_iter().map(|n| n.data).collect(),
        ))
    }

    pub fn sinks(&self) -> impl Iterator<Item = PortDiff<G>> + '_ {
        self.0.sinks().iter().map(|n| n.clone().into())
    }

    pub fn get_diff(&self, id: NodeId<G>) -> PortDiff<G> {
        self.0.get_node_rc(id.into()).into()
    }

    pub fn merge(&mut self, other: Self) {
        self.0.merge(other.0);
    }

    pub fn inner(&self) -> &relrc::GraphView<PortDiffData<G>, EdgeData<G>> {
        &self.0
    }

    pub fn lowest_common_ancestors(graphs: &[Self]) -> impl Iterator<Item = NodeId<G>> + '_ {
        relrc::GraphView::lowest_common_ancestors(graphs).map(|n| n.into())
    }
}

impl<G: Graph> Borrow<relrc::GraphView<PortDiffData<G>, EdgeData<G>>> for GraphView<G> {
    fn borrow(&self) -> &relrc::GraphView<PortDiffData<G>, EdgeData<G>> {
        &self.0
    }
}
