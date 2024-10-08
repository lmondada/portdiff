mod extract;
mod rewrite;
mod serial_edge_data;
mod squash;
// mod traverser;

pub use extract::IncompatiblePortDiff;
pub use rewrite::InvalidRewriteError;

use std::{
    cmp,
    collections::{BTreeSet, HashMap},
    fmt::{self, Debug},
    hash::Hash,
    ops::Deref,
};

use crate::{
    graph::Graph,
    port::{BoundPort, BoundaryIndex, BoundarySite, Port},
    subgraph::Subgraph,
    NodeId,
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
    fn try_with_parents(
        data: PortDiffData<G>,
        parents: Vec<(PortDiff<G>, EdgeData<G>)>,
    ) -> Result<Self, IncompatiblePortDiff> {
        if !are_compatible(&parents) {
            return Err(IncompatiblePortDiff);
        }
        Ok(Self {
            data: RelRc::with_parents(data, parents.into_iter().map(|(p, e)| (p.data, e))),
        })
    }

    pub fn as_ptr(&self) -> PortDiffPtr<G> {
        RelRc::as_ptr(&self.data)
    }
}

/// Check that a new PortDiff as a child of parents is valid
///
/// We check two things:
///  - edges outgoing from the same parent are compatible.
///  - all parents are compatible with each other.
fn are_compatible<G: Graph>(parents: &[(PortDiff<G>, EdgeData<G>)]) -> bool {
    let mut parents_map: HashMap<_, Vec<_>> = HashMap::new();
    for (parent, edge_data) in parents {
        parents_map
            .entry(parent.clone())
            .or_default()
            .push(edge_data);
    }
    // The diffs up to the parents must be valid...
    let Ok(graph) = PortDiff::try_merge(parents_map.keys().cloned()) else {
        return false;
    };
    // ...and remain valid when adding the edges to the diff graph.
    for (diff, edges) in parents_map {
        let n: NodeId<_> = (&diff).into();
        let graph_edges = graph
            .inner()
            .outgoing_edges(n.0)
            .map(|e| graph.inner().get_edge(e).value());
        if !EdgeData::are_compatible(edges.iter().copied().chain(graph_edges)) {
            return false;
        }
    }
    true
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
    pub(crate) graph: G,
    /// The boundary of the subgraph.
    ///
    /// Each boundary port of `graph` maps to a port in one of the parents,
    /// reachable by following the `IncomingEdgeIndex`.
    pub(crate) boundary: Vec<(BoundarySite<G>, IncomingEdgeIndex)>,
    /// Optionally an integer value associated with the diff. TODO: make this generic (or move out)
    pub(crate) value: Option<usize>,
}

/// The incoming edge at a portdiff, given by its index.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, From, Into, Serialize, Deserialize,
)]
pub struct IncomingEdgeIndex(usize);

#[derive_where(Clone; G: Graph)]
#[derive_where(Debug; G: Graph, G::Node: Debug, G::Edge: Debug)]
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

    /// The subgraph that is rewritten by this edge.    
    pub fn subgraph(&self) -> &Subgraph<G> {
        &self.subgraph
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
        Self::try_with_parents(
            PortDiffData {
                graph,
                value: None,
                boundary: Vec::new(),
            },
            vec![],
        )
        .unwrap()
    }

    pub fn try_unwrap_graph(self) -> Result<G, Self> {
        match RelRc::try_unwrap(self.data) {
            Ok(data) => Ok(data.graph),
            Err(data) => Err(PortDiff { data }),
        }
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

    pub fn all_parents(&self) -> impl Iterator<Item = Self> + '_ {
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
    pub fn opposite_ports<'a>(&self, port: Port<G>) -> impl Iterator<Item = Owned<Port<G>, G>>
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

    /// Resolve a port to a concrete port.
    ///
    /// In general, ports may refer to "BoundarySite::Wire"s, which are
    /// phantom objects (noops) that link two edges together. This function
    /// resolves such ports to a "concrete" port in a graph by following the
    /// wire until it finds a real port. As a consequence, this may return 0, 1
    /// or multiple ports.
    ///
    /// If the port is already a concrete port, it is returned as is.
    pub fn resolve_port(&self, port: Port<G>) -> Vec<Owned<Port<G>, G>> {
        let boundary = match port {
            Port::Boundary(index) => index,
            port @ Port::Bound(..) => {
                return vec![Owned {
                    data: port,
                    owner: self.clone(),
                }]
            }
        };
        match self.boundary_site(boundary) {
            BoundarySite::Site(..) => vec![Owned {
                data: Port::Boundary(boundary),
                owner: self.clone(),
            }],
            &BoundarySite::Wire { id, end } => {
                let opp_site = BoundarySite::Wire {
                    id,
                    end: end.opposite(),
                };
                let Some(bd_index) = self
                    .boundary_iter()
                    .filter(|&bd| self.boundary_site(bd) == &opp_site)
                    .at_most_one()
                    .ok()
                    .expect("found more than one wire end")
                else {
                    return Vec::new();
                };
                // Resolve recursively
                self.opposite_ports(Port::Boundary(bd_index))
                    .into_iter()
                    .flat_map(|Owned { data, owner }| owner.resolve_port(data))
                    .collect()
            }
        }
    }

    pub fn boundary_iter(&self) -> impl Iterator<Item = BoundaryIndex> {
        (0..self.boundary.len()).map_into()
    }

    /// The boundary port at `boundary`, if it is a site.
    pub fn boundary_site(&self, boundary: BoundaryIndex) -> &BoundarySite<G> {
        &self.boundary[usize::from(boundary)].0
    }

    pub fn is_compatible(&self, other: &Self) -> bool {
        PortDiff::are_compatible([self, other])
    }

    pub fn n_boundary_ports(&self) -> usize {
        self.boundary.len()
    }

    /// All the ports of descendants of `self` that map to `port`.
    pub fn descendants(&self, port: BoundPort<G::Edge>) -> impl Iterator<Item = Owned<Port<G>, G>> {
        DescendantsIter::new(port, self.clone())
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

struct DescendantsIter<G: Graph> {
    curr_ports: Vec<Owned<Port<G>, G>>,
}

impl<G: Graph> DescendantsIter<G> {
    fn new(port: impl Into<Port<G>>, owner: PortDiff<G>) -> Self {
        let curr_ports = vec![Owned {
            data: port.into(),
            owner,
        }];
        Self { curr_ports }
    }
}

impl<G: Graph> Iterator for DescendantsIter<G> {
    type Item = Owned<Port<G>, G>;

    fn next(&mut self) -> Option<Self::Item> {
        let port = self.curr_ports.pop()?;
        self.curr_ports
            .extend(port.owner.all_outgoing().iter().filter_map(|e| {
                let port = e.value().map_to_child(&port.data)?;
                let owner = e.target().clone().into();
                Some(Owned {
                    data: Port::from(port),
                    owner,
                })
            }));
        Some(port)
    }
}

impl<G: Graph> PortDiffData<G> {
    /// The replacement graph of the diff.
    pub fn graph(&self) -> &G {
        &self.graph
    }

    /// The value of the diff, if it is set.
    pub fn value(&self) -> Option<usize> {
        self.value
    }
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
#[derive_where(Debug; G: Graph, D: Debug)]
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

    use crate::{port::EdgeEnd, PortDiffGraph};

    use super::*;

    pub(crate) type TestPortDiff = PortDiff<PortGraph>;

    impl TestPortDiff {
        pub(crate) fn identity_subgraph(
            &self,
            nodes: impl IntoIterator<Item = NodeIndex>,
        ) -> TestPortDiff {
            let graph = self.graph.clone();
            let nodes = nodes.into_iter().collect();
            self.rewrite_induced(&nodes, graph, |p| {
                Owned::new(p, self.clone()).site().unwrap().into()
            })
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
                let old_site = Owned::new(p, root.clone()).site().unwrap();
                Site {
                    node: node_map[&old_site.node],
                    port: old_site.port,
                }
                .into()
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
                .into()
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
                .into()
            })
            .unwrap()
        };
        [root, child_1, child_2]
    }

    #[fixture]
    pub(crate) fn parent_two_children_overlapping_diffs() -> [TestPortDiff; 3] {
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
            let new_n2 = rhs.add_node(0, 0);
            let child_nodes = BTreeSet::from_iter([n0, n1, n2]);
            root.rewrite_induced(&child_nodes, rhs, |p| {
                Site {
                    node: new_n2,
                    port: Owned::new(p, root.clone()).site().unwrap().port,
                }
                .into()
            })
            .unwrap()
        };
        let child_2 = {
            let mut rhs = PortGraph::new();
            let new_n1 = rhs.add_node(0, 0);
            let child_nodes = BTreeSet::from_iter([n1, n2, n3]);
            root.rewrite_induced(&child_nodes, rhs, |p| {
                Site {
                    node: new_n1,
                    port: Owned::new(p, root.clone()).site().unwrap().port,
                }
                .into()
            })
            .unwrap()
        };
        [root, child_1, child_2]
    }

    #[rstest]
    fn serialize_parent_child(parent_child_diffs: [TestPortDiff; 2]) {
        let [_, child] = parent_child_diffs;
        let graph = PortDiffGraph::from_sinks(vec![child]);
        let serialized = serde_json::to_string_pretty(&graph).unwrap();
        insta::assert_snapshot!(serialized);
    }

    #[ignore = "TODO this is currently not deterministic"]
    #[rstest]
    fn serialize_parent_two_children(parent_two_children_diffs: [TestPortDiff; 3]) {
        let [_, child_1, child_2] = parent_two_children_diffs;
        let graph = PortDiffGraph::from_sinks(vec![child_1, child_2]);
        let serialized = serde_json::to_string_pretty(&graph).unwrap();
        insta::assert_snapshot!(serialized);
    }

    #[ignore = "TODO this is currently not deterministic"]
    #[rstest]
    fn serialize_parent_two_children_overlapping(
        parent_two_children_overlapping_diffs: [TestPortDiff; 3],
    ) {
        let [_, child_1, child_2] = parent_two_children_overlapping_diffs;
        let graph = PortDiffGraph::from_sinks(vec![child_1, child_2]);
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

    impl Graph for () {
        type Node = usize;

        type Edge = ();

        type PortLabel = ();

        fn nodes_iter(&self) -> impl Iterator<Item = Self::Node> + '_ {
            [].into_iter()
        }

        fn edges_iter(&self) -> impl Iterator<Item = Self::Edge> + '_ {
            [].into_iter()
        }

        fn get_port_site(
            &self,
            bound_port: BoundPort<Self::Edge>,
        ) -> Site<Self::Node, Self::PortLabel> {
            Site { node: 0, port: () }
        }

        fn get_bound_ports(
            &self,
            site: Site<Self::Node, Self::PortLabel>,
        ) -> impl Iterator<Item = BoundPort<Self::Edge>> + '_ {
            [].into_iter()
        }

        fn get_sites(
            &self,
            node: Self::Node,
        ) -> impl Iterator<Item = Site<Self::Node, Self::PortLabel>> + '_ {
            [].into_iter()
        }

        fn link_sites(
            &mut self,
            left: Site<Self::Node, Self::PortLabel>,
            right: Site<Self::Node, Self::PortLabel>,
        ) {
        }

        fn add_subgraph(
            &mut self,
            graph: &Self,
            nodes: &BTreeSet<Self::Node>,
        ) -> BTreeMap<Self::Node, Self::Node> {
            [].into()
        }
    }

    #[test]
    fn test_compatible() {
        let root = PortDiff::<()>::from_graph(());
        let create_child = |parents: Vec<(PortDiff<()>, Vec<usize>)>| {
            let parents = parents
                .into_iter()
                .map(|(diff, vec)| {
                    let subgraph = Subgraph::new(&(), BTreeSet::from_iter(vec), Default::default());
                    (
                        diff,
                        EdgeData {
                            subgraph,
                            port_map: Default::default(),
                        },
                    )
                })
                .collect_vec();
            PortDiff::try_with_parents(
                PortDiffData {
                    graph: (),
                    boundary: Default::default(),
                    value: None,
                },
                parents,
            )
        };
        let c1 = create_child(vec![(root.clone(), vec![0, 1, 2])]).unwrap();
        create_child(vec![(root.clone(), vec![0]), (c1.clone(), vec![0, 1])]).unwrap_err();
        let c2 = create_child(vec![(root.clone(), vec![1, 2, 3])]).unwrap();
        let c3 = create_child(vec![(root.clone(), vec![0]), (c2.clone(), vec![0, 1])]).unwrap();
        create_child(vec![(c1.clone(), vec![0]), (c3.clone(), vec![0, 1])]).unwrap_err();
        let c4 = create_child(vec![(root.clone(), vec![0]), (c2.clone(), vec![0, 1])]).unwrap();
        create_child(vec![(root.clone(), vec![0]), (c3.clone(), vec![0, 1])]).unwrap_err();
        create_child(vec![(c3.clone(), vec![0]), (c3.clone(), vec![0, 1])]).unwrap_err();
        println!("before");
        create_child(vec![(c4.clone(), vec![0]), (c2.clone(), vec![2])]).unwrap();
        create_child(vec![(c4.clone(), vec![0]), (c2.clone(), vec![2, 1])]).unwrap_err();
    }
}
