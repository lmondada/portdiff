use std::borrow::Borrow;

use derive_more::{From, Into};
use derive_where::derive_where;
use itertools::Itertools;
use petgraph::visit::{EdgeRef, IntoEdges};
use relrc::{edge::InnerEdgeData, graph_view::RelRcGraphSerializer, RelRcGraph};
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
pub struct NodeId<G: Graph>(pub(crate) relrc::NodeId<PortDiffData<G>, EdgeData<G>>);

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

    /// Merge two graphs.
    ///
    /// If `strategy` is `MergeStrategy::IgnoreConflicts`, conflicting edges are
    /// ignored. If `strategy` is `MergeStrategy::FailOnConflicts`, conflicting
    /// edges cause an error and the merge is not performed.
    pub fn merge(
        &mut self,
        other: Self,
        strategy: MergeStrategy,
    ) -> Result<(), IncompatiblePortDiff> {
        let merge_callback =
            |_, self_edges: &[&InnerEdgeData<_, _>], other_edges: &[&InnerEdgeData<_, _>]| {
                match strategy {
                    MergeStrategy::IgnoreConflicts => Ok(()),
                    MergeStrategy::FailOnConflicts => {
                        if !EdgeData::are_compatible(
                            self_edges
                                .iter()
                                .chain(other_edges.iter())
                                .map(|e| e.value()),
                        ) {
                            Err(IncompatiblePortDiff)
                        } else {
                            Ok(())
                        }
                    }
                }
            };
        self.0
            .merge(other.0, merge_callback)
            .map_err(|_| IncompatiblePortDiff)
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

    /// Set the diff values and create a new `PortDiffGraph`.
    ///
    /// The returned graph is identical to `self`, except with the diff values
    /// set to the values returned by `f`.
    pub fn map_value(&self, f: impl Fn(&PortDiffData<G>) -> Option<usize>) -> PortDiffGraph<G> {
        PortDiffGraph(self.0.map(
            |n| PortDiffData {
                value: f(n),
                ..n.clone()
            },
            |e| e.clone(),
        ))
    }
}

impl<G: Graph> Borrow<RelRcGraph<PortDiffData<G>, EdgeData<G>>> for PortDiffGraph<G> {
    fn borrow(&self) -> &RelRcGraph<PortDiffData<G>, EdgeData<G>> {
        &self.0
    }
}

/// Strategy for merging two graphs.
pub enum MergeStrategy {
    /// Ignore conflicts and merge the graphs.
    IgnoreConflicts,
    /// Fail if conflicts are detected.
    FailOnConflicts,
}
