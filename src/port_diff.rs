mod boundary;
mod merge;
mod rewrite;

pub use merge::MergeType;

use std::{
    cell::{Ref, RefCell},
    collections::BTreeSet,
    rc::{Rc, Weak},
};

use crate::edges::{
    AncestorEdge, BoundaryEdge, DescendantEdge, DescendantEdges, EdgeEnd, InternalEdge,
};
use itertools::Itertools;

use crate::{EdgeEndType, Port, PortEdge};

use self::boundary::compute_boundary;

#[derive(Debug)]
pub struct PortDiff<V, P> {
    data: Rc<PortDiffData<V, P>>,
}

type PortDiffPtr<V, P> = *const PortDiffData<V, P>;

impl<V, P> PortDiff<V, P> {
    fn new(data: PortDiffData<V, P>) -> Self
    where
        V: Clone,
    {
        let ret = Self {
            data: Rc::new(data),
        };
        // Record `ret` as a descendant at the ancestors
        ret.record_descendant();
        ret
    }

    pub(crate) fn as_ptr(&self) -> PortDiffPtr<V, P> {
        Rc::as_ptr(&self.data)
    }

    /// Add a weak ref to `desc` at all its ancestors
    fn record_descendant(&self)
    where
        V: Clone,
    {
        for boundary in self.boundary_edges() {
            let ancestor_edge = self.get_ancestor_edge(&boundary);
            let descendant_edge =
                DescendantEdge::new(&self, boundary, ancestor_edge.used_vertices().clone());
            ancestor_edge
                .owner()
                .add_descendant(descendant_edge, ancestor_edge.edge_end());
        }
    }
}

impl<V, P> Clone for PortDiff<V, P> {
    fn clone(&self) -> Self {
        Self {
            data: self.data.clone(),
        }
    }
}

impl<V, P> PartialEq for PortDiff<V, P> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
    }
}

impl<V, P> Eq for PortDiff<V, P> {}

#[derive(Clone, Debug)]
pub struct WeakPortDiff<V, P> {
    data: Weak<PortDiffData<V, P>>,
}

impl<V, P> WeakPortDiff<V, P> {
    fn new(data: Weak<PortDiffData<V, P>>) -> Self {
        Self { data }
    }

    pub fn upgrade(&self) -> Option<PortDiff<V, P>> {
        Some(PortDiff {
            data: self.data.upgrade()?,
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PortDiffData<V, P> {
    /// The internal edges
    edges: Vec<PortEdge<V, P>>,
    /// The boundary ports of the boundary edges
    boundary_ports: Vec<Port<V, P>>,
    /// Map boundary edges to an ancestor where that edge is not a boundary
    boundary_anc: Vec<AncestorEdge<V, P>>,
    /// The reverse: Map internal edges to the set of descendants where that
    /// edge is a boundary
    boundary_desc: RefCell<Vec<DescendantEdges<V, P>>>,
}

impl<V: Eq + Clone + Ord, P: Clone> PortDiff<V, P> {
    /// Create a diff with no boundary.
    ///
    /// This will be a "root" in the diff hierarchy, as it has no ancestors.
    pub fn with_no_boundary(edges: Vec<PortEdge<V, P>>) -> Self {
        let boundary_desc = RefCell::new(vec![DescendantEdges::default(); edges.len()]);
        Self::new(PortDiffData {
            edges,
            boundary_ports: vec![],
            boundary_anc: vec![],
            boundary_desc,
        })
    }

    /// Create a new port diff on `parent` with node `root`
    pub fn with_root(root: V, parent: &Self) -> Self {
        Self::with_nodes([root], parent)
    }

    pub fn with_nodes(nodes: impl IntoIterator<Item = V>, parent: &Self) -> Self {
        let nodes: BTreeSet<_> = nodes.into_iter().collect();
        // Keep edges with both ends in the nodes set
        let edges = parent
            .data
            .edges
            .iter()
            .filter(|e| nodes.contains(&e.left.node) && nodes.contains(&e.right.node))
            .cloned()
            .collect_vec();

        let boundary = compute_boundary(&nodes, &[parent]);
        let boundary_desc = RefCell::new(vec![DescendantEdges::default(); edges.len()]);

        Self::new(PortDiffData {
            edges,
            boundary_ports: boundary.ports().cloned().collect_vec(),
            boundary_anc: boundary.into_ancestors(),
            boundary_desc,
        })
    }

    pub fn extract(&self) -> Vec<PortEdge<V, P>>
    where
        V: Clone + std::fmt::Debug,
        P: Clone + Eq + std::fmt::Debug,
    {
        if self.data.boundary_ports.is_empty() {
            return self.data.edges.clone();
        }
        let mut expanded = self.clone();
        while let Some(boundary) = expanded.boundary_edges().next() {
            let Some(new_expanded) = expanded.expand(boundary).next() else {
                continue;
            };
            expanded = new_expanded;
        }
        expanded.data.edges.clone()
    }

    /// Traverse a boundary edge and list all possible opposite edge ends
    fn find_opposite_end(&self, boundary: BoundaryEdge) -> impl Iterator<Item = (Self, EdgeEnd)> {
        let ancestor_edge = self.get_ancestor_edge(&boundary).opposite();
        // The other end can be at the ancestor...
        let ancestor = [(ancestor_edge.owner().clone(), ancestor_edge.edge_end())];
        // Or any of its descendants
        let descendants = ancestor_edge
            .get_descendant_edges()
            .into_iter()
            // Only keep descendant edges with no intersecting exclude_vertices
            .filter(|e| ancestor_edge.is_compatible(e))
            .filter_map(|e| Some((e.owner()?, e.edge_end())))
            .collect_vec();
        ancestor.into_iter().chain(descendants)
    }
}

impl<V, P> PortDiff<V, P> {
    pub fn n_boundary_edges(&self) -> usize {
        self.data.boundary_ports.len()
    }

    pub fn n_internal_edges(&self) -> usize {
        self.data.edges.len()
    }

    pub fn boundary_edges(&self) -> impl Iterator<Item = BoundaryEdge> {
        (0..self.n_boundary_edges()).map(BoundaryEdge)
    }

    pub fn boundary_edge(&self, edge: &BoundaryEdge) -> &Port<V, P> {
        let &BoundaryEdge(index) = edge;
        &self.data.boundary_ports[index]
    }

    pub fn internal_edges(&self) -> impl Iterator<Item = InternalEdge> {
        (0..self.n_internal_edges()).map(InternalEdge)
    }

    pub fn internal_edge(&self, edge: &InternalEdge) -> &PortEdge<V, P> {
        let &InternalEdge(index) = edge;
        &self.data.edges[index]
    }

    pub(super) fn port(&self, edge_end: EdgeEnd) -> &Port<V, P> {
        match edge_end {
            EdgeEnd::B(b_edge) => self.boundary_edge(&b_edge),
            EdgeEnd::I(i_edge, end_type) => match end_type {
                EdgeEndType::Left => &self.internal_edge(&i_edge).left,
                EdgeEndType::Right => &self.internal_edge(&i_edge).right,
            },
        }
    }

    pub fn degree(&self, node: &V) -> usize
    where
        V: Eq,
    {
        self.data
            .edges
            .iter()
            .filter(|e| &e.left.node == node || &e.right.node == node)
            .count()
    }

    pub fn find_edge(&self, edge: &PortEdge<V, P>) -> Option<InternalEdge>
    where
        V: Eq,
        P: Eq,
    {
        self.data
            .edges
            .iter()
            .position(|e| e == edge)
            .map(InternalEdge)
    }

    pub fn vertices(&self) -> impl Iterator<Item = &V>
    where
        V: Eq + Ord,
    {
        self.data
            .edges
            .iter()
            .flat_map(|e| [&e.left.node, &e.right.node])
            .chain(self.data.boundary_ports.iter().map(|p| &p.node))
            .sorted()
            .dedup()
    }

    fn get_ancestor_edge(&self, edge: &BoundaryEdge) -> &AncestorEdge<V, P> {
        let &BoundaryEdge(index) = edge;
        &self.data.boundary_anc[index]
    }

    pub fn get_ancestor(&self, edge: &BoundaryEdge) -> &PortDiff<V, P> {
        self.get_ancestor_edge(edge).owner()
    }

    pub(crate) fn get_descendant_edges(
        &self,
        edge: &InternalEdge,
        end: EdgeEndType,
    ) -> Ref<[DescendantEdge<V, P>]> {
        let &InternalEdge(index) = edge;
        // Before returning the list, take the opportunity to remove any old
        // weak refs
        self.data.boundary_desc.borrow_mut()[index].remove_empty_refs();

        let boundary_desc = self.data.boundary_desc.borrow();
        match end {
            EdgeEndType::Left => Ref::map(boundary_desc, |r| r[index].left.as_slice()),
            EdgeEndType::Right => Ref::map(boundary_desc, |r| r[index].right.as_slice()),
        }
    }

    pub fn has_any_descendants(&self) -> bool {
        self.data
            .boundary_desc
            .borrow()
            .iter()
            .any(|r| !r.is_empty())
    }

    #[cfg(test)]
    fn find_boundary_edge(&self, node: &V, port: &P) -> Option<BoundaryEdge>
    where
        V: Eq,
        P: Eq,
    {
        self.boundary_edges().find(|edge| {
            let Port { node: n, port: p } = self.boundary_edge(edge);
            n == node && p == port
        })
    }

    fn add_descendant(&self, descendant: DescendantEdge<V, P>, edge_end: EdgeEnd) {
        let EdgeEnd::I(edge, end) = edge_end else {
            panic!("Can only add descendant edges to internal edges");
        };
        let InternalEdge(index) = edge;
        let mut desc_map = self.data.boundary_desc.borrow_mut();
        match end {
            EdgeEndType::Left => desc_map[index].left.push(descendant),
            EdgeEndType::Right => desc_map[index].right.push(descendant),
        }
    }

    pub(crate) fn downgrade(&self) -> WeakPortDiff<V, P> {
        WeakPortDiff::new(Rc::downgrade(&self.data))
    }
}

#[cfg(test)]
mod tests {
    use rstest::{fixture, rstest};
    use uuid::Uuid;

    use crate::UniqueVertex;

    use super::*;

    pub(crate) type TestPortDiff = PortDiff<UniqueVertex, i32>;

    pub(crate) fn test_nodes() -> [UniqueVertex; 4] {
        [
            UniqueVertex::from_id(Uuid::from_u128(0)),
            UniqueVertex::from_id(Uuid::from_u128(1)),
            UniqueVertex::from_id(Uuid::from_u128(2)),
            UniqueVertex::from_id(Uuid::from_u128(3)),
        ]
    }

    #[fixture]
    pub(crate) fn root_diff() -> TestPortDiff {
        let nodes = test_nodes();
        let new_port = |n, i| Port {
            node: nodes[n],
            port: i,
        };
        let ports_0 = (0..3).map(|i| new_port(0, i)).collect_vec();
        let ports_1 = (0..4).map(|i| new_port(1, i)).collect_vec();
        let ports_2 = (0..4).map(|i| new_port(2, i)).collect_vec();
        let ports_3 = (0..3).map(|i| new_port(3, i)).collect_vec();
        let edges_0_1 = ports_0
            .iter()
            .zip(&ports_1)
            .map(|(l, r)| PortEdge {
                left: l.clone(),
                right: r.clone(),
            })
            .collect_vec();
        let edges_2_3 = ports_2
            .iter()
            .zip(&ports_3)
            .map(|(l, r)| PortEdge {
                left: l.clone(),
                right: r.clone(),
            })
            .collect_vec();
        let edges_1_2 = vec![PortEdge {
            left: ports_1[3].clone(),
            right: ports_2[3].clone(),
        }];
        let edges = edges_0_1
            .into_iter()
            .chain(edges_1_2)
            .chain(edges_2_3)
            .collect_vec();
        PortDiff::with_no_boundary(edges)
    }

    // #[rstest]
    // fn test_port_diff(root_diff: TestPortDiff) {
    //     let nodes = test_nodes();
    //     let just_1 = PortDiff::with_root(nodes[1], &root_diff);
    //     assert_eq!(just_1.n_boundary_edges(), 4);
    //     assert_eq!(just_1.n_internal_edges(), 0);
    //     let child_1_2 = {
    //         let edge = just_1.find_boundary_edge(&nodes[1], &3).unwrap();
    //         let expansion_opts = just_1.expand(edge).collect_vec();
    //         assert_eq!(expansion_opts.len(), 1);
    //         expansion_opts.into_iter().next().unwrap()
    //     };
    //     assert_eq!(child_1_2.n_boundary_edges(), 3 + 3);
    //     assert_eq!(child_1_2.n_internal_edges(), 1);

    //     // Check boundary
    //     let boundary = BTreeSet::from_iter(child_1_2.data.boundary_ports.clone());
    //     let exp_boundary = BTreeSet::from_iter(
    //         (0..3)
    //             .map(|i| Port {
    //                 node: nodes[1],
    //                 port: i,
    //             })
    //             .chain((0..3).map(|i| Port {
    //                 node: nodes[2],
    //                 port: i,
    //             })),
    //     );
    //     assert_eq!(boundary, exp_boundary);

    //     // Check internal edges
    //     assert_eq!(
    //         child_1_2.data.edges,
    //         [PortEdge {
    //             left: Port {
    //                 node: nodes[1],
    //                 port: 3
    //             },
    //             right: Port {
    //                 node: nodes[2],
    //                 port: 3
    //             },
    //         }]
    //     );
    // }

    #[rstest]
    fn test_with_nodes(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        assert_eq!(child_a.n_boundary_edges(), 1);
        assert_eq!(child_a.n_internal_edges(), 3);
        assert!(!root_diff.data.boundary_desc.borrow()[3].left.is_empty());
        assert!(root_diff.data.boundary_desc.borrow()[3].right.is_empty());

        let _ = PortDiff::with_nodes([nodes[2], nodes[3]], &root_diff);
        assert!(!root_diff.data.boundary_desc.borrow()[3].right.is_empty());
    }
}
