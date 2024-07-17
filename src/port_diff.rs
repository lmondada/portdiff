mod extract;
mod rewrite;

use std::{
    cell::{Ref, RefCell},
    cmp,
    collections::{BTreeMap, BTreeSet},
    fmt::{self, Debug},
    hash::Hash,
    ops::Deref,
    rc::{Rc, Weak},
};

use crate::{
    graph::Graph,
    port::{BoundPort, ChildPort, ParentPort, Port},
};
use itertools::Itertools;

use crate::port::UnboundPort;

pub struct PortDiff<G: Graph> {
    data: Rc<PortDiffData<G>>,
}

type PortDiffPtr<G> = *const PortDiffData<G>;

impl<G: Graph> PortDiff<G> {
    fn new(data: PortDiffData<G>) -> Self {
        let ret = Self {
            data: Rc::new(data),
        };
        // Record `ret` as a descendant at the ancestors
        ret.register_child();
        ret
    }

    pub(crate) fn as_ptr(&self) -> PortDiffPtr<G> {
        Rc::as_ptr(&self.data)
    }

    /// Add a weak ref to `self` at all its parents
    fn register_child(&self) {
        for (child_port, ParentPort { parent, port }) in &self.boundary {
            parent.add_child(
                *port,
                ChildPort {
                    child: self.downgrade(),
                    port: child_port.clone(),
                },
            );
        }
    }
}

impl<G: Graph> Clone for PortDiff<G> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<G: Graph> PartialEq for PortDiff<G> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
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

impl<G: Graph> PartialEq for WeakPortDiff<G> {
    fn eq(&self, other: &Self) -> bool {
        let Some(self_data) = self.data.upgrade() else {
            return false;
        };
        let Some(other_data) = other.data.upgrade() else {
            return false;
        };
        Rc::ptr_eq(&self_data, &other_data)
    }
}

impl<G: Graph> Eq for WeakPortDiff<G> {}

#[derive(Clone, Debug)]
pub struct WeakPortDiff<G: Graph> {
    data: Weak<PortDiffData<G>>,
}

impl<G: Graph> WeakPortDiff<G> {
    fn new(data: Weak<PortDiffData<G>>) -> Self {
        Self { data }
    }

    pub fn upgrade(&self) -> Option<PortDiff<G>> {
        Some(PortDiff {
            data: self.data.upgrade()?,
        })
    }

    pub fn is_upgradable(&self) -> bool {
        self.data.strong_count() > 0
    }
}

/// Uniquely identify nodes in the rewrite history by their node and the graph
/// they belong to.
///
/// TODO: this should be deterministic and identical across multiple processes,
/// so replace the graph pointer with a hash of the rewrite history.
pub struct UniqueNodeId<G: Graph> {
    node: G::Node,
    owner: PortDiff<G>,
}

#[derive(Debug)]
pub enum PortDiffEdge<G: Graph> {
    Internal {
        owner: PortDiff<G>,
        edge: G::Edge,
    },
    Boundary {
        left_owner: PortDiff<G>,
        left_port: UnboundPort<G::Node, G::PortLabel>,
        right_owner: PortDiff<G>,
        right_port: UnboundPort<G::Node, G::PortLabel>,
    },
}

impl<G: Graph> Debug for UniqueNodeId<G>
where
    G::Node: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UniqueNodeId {{ node: {:?}, owner: {:?} }}",
            self.node, self.owner
        )
    }
}

impl<G: Graph> PartialEq for UniqueNodeId<G> {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node && self.owner == other.owner
    }
}

impl<G: Graph> Eq for UniqueNodeId<G> {}

impl<G: Graph> PartialOrd for UniqueNodeId<G> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<G: Graph> Ord for UniqueNodeId<G> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        let as_tuple = |o: &Self| (o.node, o.owner.as_ptr());
        as_tuple(self).cmp(&as_tuple(other))
    }
}

impl<G: Graph> Clone for UniqueNodeId<G> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            owner: self.owner.clone(),
        }
    }
}

impl<G: Graph> UniqueNodeId<G> {
    fn new(node: G::Node, graph: PortDiff<G>) -> Self {
        Self { node, owner: graph }
    }
}

type Boundary<G> =
    BTreeMap<UnboundPort<<G as Graph>::Node, <G as Graph>::PortLabel>, ParentPort<G>>;

#[derive(Clone)]
pub struct PortDiffData<G: Graph> {
    /// The internal graph
    graph: G,
    /// The boundary is a map from unbound ports in `graph` to bound ports
    /// in a parent graph.
    ///
    /// Currently every (node, port label) pair can have at most one boundary port.
    boundary: Boundary<G>,
    /// The reverse boundary map, i.e. map bound ports in `graph` to all
    /// unbound ports in children `graphs`.
    children: RefCell<BTreeMap<BoundPort<G::Edge>, Vec<ChildPort<G>>>>,
    /// Nodes that have been deleted in the rewrite history
    exclude_nodes: BTreeSet<UniqueNodeId<G>>,
}

impl<G: Graph> Deref for PortDiff<G> {
    type Target = PortDiffData<G>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<G: Graph> From<PortDiffData<G>> for PortDiff<G> {
    fn from(data: PortDiffData<G>) -> Self {
        Self::new(data)
    }
}

impl<G: Graph> Hash for PortDiff<G> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ptr().hash(state);
    }
}

impl<G: Graph> PortDiff<G> {
    /// Create a diff with no boundary.
    ///
    /// This will be a "root" in the diff hierarchy, as it has no ancestors.
    pub fn from_graph(graph: G) -> Self {
        Self::new(PortDiffData {
            graph,
            boundary: BTreeMap::new(),
            children: RefCell::new(BTreeMap::new()),
            exclude_nodes: BTreeSet::new(),
        })
    }

    pub fn graph(&self) -> &G {
        &self.data.graph
    }

    fn exclude_nodes<'a>(&'a self, parent: &'a PortDiff<G>) -> impl Iterator<Item = G::Node> + 'a {
        self.exclude_nodes
            .iter()
            .filter_map(move |n| (&n.owner == parent).then_some(n.node))
    }

    /// Traverse a boundary edge and list all possible opposite edge ends
    ///
    /// There is no guarantee that the opposite end does not clash with `self`.
    ///
    /// TODO: return them in toposort order
    fn find_opposite_end(
        &self,
        boundary: &UnboundPort<G::Node, G::PortLabel>,
    ) -> impl Iterator<Item = Port<G>> {
        let parent_port = self.parent_port(boundary);
        // The other end can be at the parent...
        let parent_opposite: Port<_> = parent_port.opposite().into();
        // Or any of its descendants
        let children = parent_port
            .children()
            .into_iter()
            .filter_map(|child| child.upgrade())
            .collect_vec();
        Some(parent_opposite).into_iter().chain(children)
    }

    fn parent_port(&self, boundary: &UnboundPort<G::Node, G::PortLabel>) -> &ParentPort<G> {
        &self.boundary[&boundary]
    }

    pub fn is_compatible(&self, other: &Self) -> bool {
        PortDiff::are_compatible([self, other])
    }

    pub fn n_boundary_ports(&self) -> usize {
        self.boundary.len()
    }

    fn retain_upgradable_children(&self, port: BoundPort<G::Edge>) {
        let mut children = self.data.children.borrow_mut();
        let port_children = children.get_mut(&port);
        if let Some(port_children) = port_children {
            port_children.retain(|child| child.is_upgradable());
        }
    }

    pub(crate) fn children(&self, port: BoundPort<G::Edge>) -> Ref<[ChildPort<G>]> {
        // Before returning the list, take the opportunity to remove any old
        // weak refs
        self.retain_upgradable_children(port);

        self.children.borrow_mut().entry(port).or_default();

        Ref::map(self.children.borrow(), |children| {
            children[&port].as_slice()
        })
    }

    pub(crate) fn all_children(&self) -> Vec<PortDiff<G>> {
        self.children
            .borrow()
            .values()
            .flat_map(|children| children.iter().flat_map(|c| c.child.upgrade()))
            .collect_vec()
    }

    pub(crate) fn parents(&self) -> impl Iterator<Item = &PortDiff<G>> {
        self.boundary.values().map(|p| &p.parent).unique()
    }

    pub fn has_any_descendants(&self) -> bool {
        self.children.borrow().values().any(|r| !r.is_empty())
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

    fn add_child(&self, port: BoundPort<G::Edge>, child_port: ChildPort<G>) {
        self.children
            .borrow_mut()
            .entry(port)
            .or_default()
            .push(child_port);
    }

    pub(crate) fn downgrade(&self) -> WeakPortDiff<G> {
        WeakPortDiff::new(Rc::downgrade(&self.data))
    }
}

#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use portgraph::{LinkMut, NodeIndex, PortGraph, PortMut, PortOffset};
    use rstest::{fixture, rstest};

    use crate::port::PortSide;

    use super::*;

    pub(crate) type TestPortDiff = PortDiff<PortGraph>;

    impl TestPortDiff {
        pub(crate) fn identity_subgraph(
            &self,
            nodes: impl IntoIterator<Item = NodeIndex>,
        ) -> TestPortDiff {
            let graph = self.graph.clone();
            let nodes = nodes.into_iter().collect();
            self.rewrite_induced(&nodes, graph, |n| n).unwrap()
        }
    }

    #[fixture]
    pub(crate) fn root_diff() -> TestPortDiff {
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
        PortDiff::from_graph(graph)
    }

    #[rstest]
    fn test_register_child(root_diff: TestPortDiff) {
        let (n0, n1, n2, _) = Graph::nodes_iter(&root_diff.graph).collect_tuple().unwrap();
        let mut rhs = PortGraph::new();
        rhs.add_node(3, 0);
        rhs.add_node(0, 3);
        let child_nodes = BTreeSet::from_iter([n1, n2]);
        let child_a = root_diff.rewrite_induced(&child_nodes, rhs, |n| n).unwrap();
        assert_eq!(child_a.n_boundary_ports(), 6);

        for (node, port_side) in [(n0, PortSide::Right), (n2, PortSide::Left)] {
            for offset in 0..3 {
                let port = BoundPort {
                    edge: (node, PortOffset::Outgoing(offset)).try_into().unwrap(),
                    port: port_side,
                };
                assert_eq!(
                    root_diff.children(port)[0].child.upgrade().unwrap(),
                    child_a
                );
            }
        }

        for port_side in [PortSide::Left, PortSide::Right] {
            let port = BoundPort {
                edge: (n1, PortOffset::Outgoing(0)).try_into().unwrap(),
                port: port_side,
            };
            assert!(root_diff.children(port).is_empty());
        }
    }
}
