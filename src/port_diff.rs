mod extract;
mod rewrite;
mod serial_edge_data;
mod squash;
// mod traverser;

pub use extract::IncompatiblePortDiff;

use std::{
    cmp,
    collections::BTreeSet,
    fmt::{self, Debug},
    hash::Hash,
    ops::Deref,
};

use crate::{
    graph::Graph,
    port::{BoundPort, BoundaryIndex, Port},
    subgraph::Subgraph,
};
use bimap::BiBTreeMap;
use derive_more::{From, Into};
use derive_where::derive_where;
use itertools::Itertools;
use relrc::RelRc;
use serde::{Deserialize, Serialize};

use crate::port::Site;

// pub use traverser::DiffTraverser;

#[derive(From)]
#[derive_where(Clone; G: Graph)]
pub struct PortDiff<G: Graph> {
    pub(crate) data: RelRc<PortDiffData<G>, EdgeData<G>>,
}

pub type PortDiffPtr<G> = *const relrc::node::InnerData<PortDiffData<G>, EdgeData<G>>;

impl<G: Graph> PortDiff<G> {
    fn new(
        data: PortDiffData<G>,
        parents: impl IntoIterator<Item = (PortDiff<G>, EdgeData<G>)>,
    ) -> Self {
        Self {
            data: RelRc::with_parents(data, parents.into_iter().map(|(p, e)| (p.data, e))),
        }
    }

    pub(crate) fn as_ptr(&self) -> PortDiffPtr<G> {
        RelRc::as_ptr(&self.data)
    }
}

impl<G: Graph> Hash for PortDiff<G> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
    }
}

impl<G: Graph> PartialEq for PortDiff<G> {
    fn eq(&self, other: &Self) -> bool {
        RelRc::ptr_eq(&self.data, &other.data)
    }
}

impl<G: Graph> Eq for PortDiff<G> {}

impl<G: Graph> Debug for PortDiff<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PortDiff {{ ptr: {:?}, n_nodes: {} }}",
            self.as_ptr(),
            self.graph.nodes_iter().count()
        )
    }
}

impl<G: Graph> PartialOrd for PortDiff<G> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<G: Graph> Ord for PortDiff<G> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "G: Serialize, G::Node: Serialize, G::PortLabel: Serialize"))]
#[serde(bound(
    deserialize = "G: Deserialize<'de>, G::Node: Deserialize<'de>, G::PortLabel: Deserialize<'de>"
))]
pub struct PortDiffData<G: Graph> {
    /// The internal graph
    graph: G,
    /// The boundary of the subgraph.
    ///
    /// Maps boundary ports of `graph` to a port in one of the parents.
    boundary: Vec<(Site<G::Node, G::PortLabel>, IncomingEdgeIndex)>,
}

/// The incoming edge at a portdiff, given by its index.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct IncomingEdgeIndex(usize);

#[derive_where(Clone; G: Graph)]
pub struct EdgeData<G: Graph> {
    /// The parent subgraph that is rewritten.
    subgraph: Subgraph<G>,
    /// Map ports in parent to boundary ports in child
    ///
    /// The domain of the map is the union of the boundary of `subgraph` and
    /// the boundary ports of `parent` that are on `subgraph.nodes`.
    port_map: BiBTreeMap<Port<G>, BoundaryIndex>,
}

impl<G: Graph> EdgeData<G> {
    fn map_to_child(&self, port: &Port<G>) -> Option<BoundaryIndex> {
        self.port_map.get_by_left(&port).copied()
    }

    fn map_to_parent(&self, boundary: BoundaryIndex) -> Option<&Port<G>> {
        self.port_map.get_by_right(&boundary)
    }

    pub(crate) fn are_compatible<'a>(edges: impl IntoIterator<Item = &'a Self>) -> bool
    where
        G: 'a,
    {
        // All vertex sets must be disjoint
        let mut all_nodes = BTreeSet::new();
        for edge in edges {
            if !edge.subgraph.nodes().iter().all(|&v| all_nodes.insert(v)) {
                return false;
            }
        }

        true
    }
}

impl<G: Graph> Deref for PortDiff<G> {
    type Target = PortDiffData<G>;

    fn deref(&self) -> &Self::Target {
        self.data.value()
    }
}

type InEdge<G> = relrc::edge::InnerEdgeData<PortDiffData<G>, EdgeData<G>>;
type OutEdge<G> = relrc::edge::Edge<PortDiffData<G>, EdgeData<G>>;

impl<G: Graph> PortDiff<G> {
    /// Create a diff with no boundary.
    ///
    /// This will be a "root" in the diff hierarchy, as it has no ancestors.
    pub fn from_graph(graph: G) -> Self {
        Self::new(
            PortDiffData {
                graph,
                boundary: Vec::new(),
            },
            [],
        )
    }

    pub fn graph(&self) -> &G {
        &self.graph
    }

    /// The i-th incoming edge
    fn incoming(&self, index: IncomingEdgeIndex) -> Option<&InEdge<G>> {
        self.data.incoming(index.0)
    }

    fn incoming_edge_index(&self, boundary: BoundaryIndex) -> Option<IncomingEdgeIndex> {
        self.boundary
            .get(usize::from(boundary))
            .map(|&(_, index)| index)
    }

    fn port_outgoing(&self, port: Port<G>) -> impl Iterator<Item = OutEdge<G>> + '_ {
        self.all_outgoing()
            .into_iter()
            .filter(move |e| e.value().map_to_child(&port).is_some())
    }

    /// All incoming edges.
    fn all_incoming(&self) -> &[InEdge<G>] {
        self.data.all_incoming()
    }

    fn all_parents(&self) -> impl Iterator<Item = Self> + '_ {
        self.data.all_parents().map(|p| p.clone().into()).unique()
    }

    /// All outgoing edges.
    fn all_outgoing(&self) -> Vec<OutEdge<G>> {
        self.data.all_outgoing()
    }

    /// Recursively follow parent ports until finding a bound port.
    pub fn bound_ancestor(&self, mut boundary: BoundaryIndex) -> Owned<BoundPort<G::Edge>, G> {
        let mut owner = self.clone();
        loop {
            match owner.parent_port(boundary) {
                Owned {
                    data: Port::Bound(port),
                    owner,
                } => {
                    return Owned { data: port, owner };
                }
                Owned {
                    data: Port::Boundary(index),
                    owner: parent,
                } => {
                    boundary = index;
                    owner = parent;
                }
            }
        }
    }

    /// The parent port of a boundary port.
    pub fn parent_port(&self, boundary: BoundaryIndex) -> Owned<Port<G>, G> {
        let edge_index = self.incoming_edge_index(boundary).unwrap();
        let edge = self.incoming(edge_index).unwrap();
        let port = edge.value().map_to_parent(boundary).unwrap().clone();
        let owner = edge.source().clone().into();
        Owned { data: port, owner }
    }

    /// Traverse a boundary edge and list all possible opposite edge ends
    ///
    /// There is no guarantee that the opposite end does not clash with `self`.
    ///
    /// TODO: return them in toposort order
    pub fn opposite_ports<'a>(&self, port: Port<G>) -> Vec<Owned<Port<G>, G>>
    where
        G: 'a,
    {
        // Find the first parent port that is an ancestor of `port`.
        let parent_port = match port {
            Port::Boundary(boundary) => self.bound_ancestor(boundary),
            Port::Bound(port) => {
                let owner = self.clone();
                Owned { data: port, owner }
            }
        };
        parent_port.owner.descendants(parent_port.data.opposite())
    }

    fn boundary_iter(&self) -> impl Iterator<Item = BoundaryIndex> {
        (0..self.boundary.len()).map_into()
    }

    pub fn boundary_site(&self, boundary: BoundaryIndex) -> &Site<G::Node, G::PortLabel> {
        &self.boundary[usize::from(boundary)].0
    }

    pub fn is_compatible(&self, other: &Self) -> bool {
        PortDiff::are_compatible([self, other])
    }

    pub fn n_boundary_ports(&self) -> usize {
        self.boundary.len()
    }

    /// All the ports of descendants of `self` that map to `port`.
    pub fn descendants(&self, port: BoundPort<G::Edge>) -> Vec<Owned<Port<G>, G>> {
        let mut curr_ports = vec![Owned {
            data: Port::from(port),
            owner: self.clone(),
        }];
        let mut all_ports = curr_ports.clone();
        while let Some(port) = curr_ports.pop() {
            all_ports.push(port.clone());
            curr_ports.extend(port.owner.all_outgoing().iter().filter_map(|e| {
                let port = e.value().map_to_child(&port.data)?;
                let owner = e.target().clone().into();
                Some(Owned {
                    data: Port::from(port),
                    owner,
                })
            }));
        }
        all_ports
    }

    pub fn all_children(&self) -> impl Iterator<Item = PortDiff<G>> + '_ {
        self.data.all_children().map(|p| p.into())
    }

    pub fn port_children(&self, port: Port<G>) -> impl Iterator<Item = PortDiff<G>> + '_ {
        self.port_outgoing(port).map(|e| e.target().clone().into())
    }

    pub fn has_any_descendants(&self) -> bool {
        self.data.n_outgoing() > 0
    }

    // #[cfg(test)]
    // fn find_boundary_edge(&self, node: &V, port: &P) -> Option<BoundaryEdge>
    // where
    //     V: Eq,
    //     P: Eq,
    // {
    //     self.boundary_edges().find(|edge| {
    //         let UnboundPort { node: n, port: p } = self.boundary_edge(edge);
    //         n == node && p == port
    //     })
    // }
}

/// A piece of data along with its owning portdiff.
///
/// This is useful for ports, node indices etc.
#[derive_where(Clone; G: Graph, D: Clone)]
#[derive_where(PartialEq; G: Graph, D: PartialEq)]
#[derive_where(Eq; G: Graph, D: Eq)]
#[derive_where(Hash; G: Graph, D: Hash)]
#[derive_where(PartialOrd; G: Graph, D: PartialOrd)]
#[derive_where(Ord; G: Graph, D: Ord)]
pub struct Owned<D, G: Graph> {
    pub data: D,
    pub owner: PortDiff<G>,
}

impl<D, G: Graph> Owned<D, G> {
    pub fn new(data: D, owner: PortDiff<G>) -> Self {
        Self { data, owner }
    }
}

impl<G: Graph> Owned<BoundPort<G::Edge>, G> {
    fn opposite(&self) -> Self {
        Self {
            data: self.data.opposite(),
            owner: self.owner.clone(),
        }
    }

    fn site(&self) -> Owned<Site<G::Node, G::PortLabel>, G> {
        Owned {
            data: self.owner.graph.get_port_site(self.data),
            owner: self.owner.clone(),
        }
    }
}

#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use portgraph::{LinkMut, NodeIndex, PortGraph, PortMut, PortOffset};
    use rstest::{fixture, rstest};

    use crate::{port::EdgeEnd, GraphView};

    use super::*;

    pub(crate) type TestPortDiff = PortDiff<PortGraph>;

    impl TestPortDiff {
        pub(crate) fn identity_subgraph(
            &self,
            nodes: impl IntoIterator<Item = NodeIndex>,
        ) -> TestPortDiff {
            let graph = self.graph.clone();
            let nodes = nodes.into_iter().collect();
            self.rewrite_induced(&nodes, graph, |p| Owned::new(p, self.clone()).site())
                .unwrap()
        }
    }

    #[fixture]
    pub(crate) fn parent_child_diffs() -> [TestPortDiff; 2] {
        let mut graph = PortGraph::new();

        let n0 = graph.add_node(0, 3);
        let n1 = graph.add_node(3, 1);
        let n2 = graph.add_node(1, 3);
        let n3 = graph.add_node(3, 0);

        for i in 0..3 {
            graph.link_nodes(n0, i, n1, i).unwrap();
            graph.link_nodes(n2, i, n3, i).unwrap();
        }
        graph.link_nodes(n1, 0, n2, 0).unwrap();
        let root = PortDiff::from_graph(graph);

        let mut rhs = PortGraph::new();
        let new_n1 = rhs.add_node(3, 0);
        let new_n2 = rhs.add_node(0, 3);
        let child_nodes = BTreeSet::from_iter([n1, n2]);
        let node_map: BTreeMap<_, _> = [(n1, new_n1), (n2, new_n2)].into_iter().collect();
        let child = root
            .rewrite_induced(&child_nodes, rhs, |p| {
                let old_site = Owned::new(p, root.clone()).site();
                Site {
                    node: node_map[&old_site.node],
                    port: old_site.port,
                }
            })
            .unwrap();
        [root, child]
    }

    #[fixture]
    pub(crate) fn parent_two_children_diffs() -> [TestPortDiff; 3] {
        let mut graph = PortGraph::new();

        let n0 = graph.add_node(0, 3);
        let n1 = graph.add_node(3, 1);
        let n2 = graph.add_node(1, 3);
        let n3 = graph.add_node(3, 0);

        for i in 0..3 {
            graph.link_nodes(n0, i, n1, i).unwrap();
            graph.link_nodes(n2, i, n3, i).unwrap();
        }
        graph.link_nodes(n1, 0, n2, 0).unwrap();
        let root = PortDiff::from_graph(graph);

        let child_1 = {
            let mut rhs = PortGraph::new();
            let new_n1 = rhs.add_node(0, 0);
            let child_nodes = BTreeSet::from_iter([n0, n1]);
            root.rewrite_induced(&child_nodes, rhs, |_| {
                // there is only one port, hard code it
                Site {
                    node: new_n1,
                    port: PortOffset::Outgoing(0),
                }
            })
            .unwrap()
        };
        let child_2 = {
            let mut rhs = PortGraph::new();
            let new_n2 = rhs.add_node(0, 0);
            let child_nodes = BTreeSet::from_iter([n2, n3]);
            root.rewrite_induced(&child_nodes, rhs, |_| {
                // there is only one port, hard code it
                Site {
                    node: new_n2,
                    port: PortOffset::Incoming(0),
                }
            })
            .unwrap()
        };
        [root, child_1, child_2]
    }

    #[rstest]
    fn serialize_parent_child(parent_child_diffs: [TestPortDiff; 2]) {
        let [_, child] = parent_child_diffs;
        let graph = GraphView::from_sinks(vec![child]);
        let serialized = serde_json::to_string_pretty(&graph).unwrap();
        insta::assert_snapshot!(serialized);
    }

    #[rstest]
    fn serialize_parent_two_children(parent_two_children_diffs: [TestPortDiff; 3]) {
        let [_, child_1, child_2] = parent_two_children_diffs;
        let graph = GraphView::from_sinks(vec![child_1, child_2]);
        let serialized = serde_json::to_string_pretty(&graph).unwrap();
        insta::assert_snapshot!(serialized);
    }

    #[rstest]
    fn test_register_child(parent_child_diffs: [TestPortDiff; 2]) {
        let [parent, child] = parent_child_diffs;
        let (n0, n1, n2, _) = Graph::nodes_iter(&parent.graph).collect_tuple().unwrap();
        assert_eq!(child.n_boundary_ports(), 6);

        for (node, port_side) in [(n0, EdgeEnd::Right), (n2, EdgeEnd::Left)] {
            for offset in 0..3 {
                let port = Port::from(BoundPort {
                    edge: (node, PortOffset::Outgoing(offset)).try_into().unwrap(),
                    end: port_side,
                });
                assert_eq!(parent.port_children(port).next().unwrap(), child);
            }
        }

        for port_side in [EdgeEnd::Left, EdgeEnd::Right] {
            let port = Port::from(BoundPort {
                edge: (n1, PortOffset::Outgoing(0)).try_into().unwrap(),
                end: port_side,
            });
            assert!(parent.port_children(port).next().is_none());
        }
    }
}
