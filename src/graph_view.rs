use std::borrow::Borrow;

use derive_more::{From, Into};
use derive_where::derive_where;
use itertools::Itertools;
use petgraph::visit::{EdgeRef, IntoEdges};
use relrc::RelRcGraph;
use serde::{Deserialize, Serialize};

use crate::{
    port_diff::{EdgeData, IncompatiblePortDiff, PortDiffData},
    Graph, PortDiff,
};

/// A view into a graph that only shows a subset of the nodes.
#[derive(From, Into, Serialize, Deserialize)]
#[derive_where(Clone, Default; G: Graph)]
#[serde(bound(
    serialize = "G: Serialize, G::Node: Serialize, G::PortLabel: Serialize, G::Edge: Serialize",
    deserialize = "G: Deserialize<'de>, G::Node: Deserialize<'de>, G::PortLabel: Deserialize<'de>, G::Edge: Deserialize<'de>"
))]
pub struct PortDiffGraph<G: Graph>(RelRcGraph<PortDiffData<G>, EdgeData<G>>);

/// A handle to a node in a graph view.
#[derive(From, Into)]
#[derive_where(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord; G: Graph)]
pub struct NodeId<G: Graph>(relrc::NodeId<PortDiffData<G>, EdgeData<G>>);

impl<'a, G: Graph> From<&'a PortDiff<G>> for NodeId<G> {
    fn from(value: &'a PortDiff<G>) -> Self {
        let node_id: relrc::NodeId<_, _> = (&value.data).into();
        node_id.into()
    }
}

impl<G: Graph> PortDiffGraph<G> {
    pub fn all_nodes(&self) -> impl Iterator<Item = NodeId<G>> + '_ {
        self.0.all_nodes().iter().copied().map(|n| n.into())
    }

    pub fn from_sinks(sinks: impl IntoIterator<Item = PortDiff<G>>) -> Self {
        Self(RelRcGraph::from_sinks(
            sinks.into_iter().map(|n| n.data).collect(),
        ))
    }

    pub fn from_sinks_while(
        sinks: impl IntoIterator<Item = PortDiff<G>>,
        predicate: impl Fn(&PortDiff<G>) -> bool,
    ) -> Self {
        Self(RelRcGraph::from_sinks_while(
            sinks.into_iter().map(|n| n.data).collect(),
            |n| predicate(&PortDiff { data: n.clone() }),
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

    pub fn inner(&self) -> &RelRcGraph<PortDiffData<G>, EdgeData<G>> {
        &self.0
    }

    pub fn lowest_common_ancestors(graphs: &[Self]) -> impl Iterator<Item = NodeId<G>> + '_ {
        RelRcGraph::lowest_common_ancestors(graphs).map(|n| n.into())
    }

    pub fn is_squashable(&self) -> bool {
        for diff_ptr in self.all_nodes() {
            // Check that its outgoing edges are compatible
            // (this must hold everywhere, but already holds elsewhere as the
            // set of outgoing edges in non-lca nodes remains unchanged).
            let edges = self.inner().edges(diff_ptr.into()).collect_vec();
            if !EdgeData::are_compatible(edges.iter().map(|e| e.weight())) {
                return false;
            }
        }
        true
    }

    /// Squash all diffs in the graph view into a single equivalent diff.
    ///
    /// Errors if `is_squashable` returns false on `self`.
    pub fn try_squash(&self) -> Result<PortDiff<G>, IncompatiblePortDiff> {
        if !self.is_squashable() {
            return Err(IncompatiblePortDiff);
        }
        let diff = PortDiff::squash(self);
        Ok(diff)
    }
}

impl<G: Graph> Borrow<RelRcGraph<PortDiffData<G>, EdgeData<G>>> for PortDiffGraph<G> {
    fn borrow(&self) -> &RelRcGraph<PortDiffData<G>, EdgeData<G>> {
        &self.0
    }
}
