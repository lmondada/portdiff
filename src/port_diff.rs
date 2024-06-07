mod boundary;
mod rewrite;

use std::{
    cell::{Ref, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

use crate::edges::{
    AncestorEdge, BoundaryEdge, DescendantEdge, DescendantEdges, EdgeEnd, InternalEdge,
};
use itertools::Itertools;

use crate::{EdgeEndType, Port, PortEdge};

use self::boundary::{compute_boundary, Boundary};

#[derive(Clone, Debug)]
pub struct PortDiff<V, P> {
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
    pub fn with_no_boundary(edges: Vec<PortEdge<V, P>>) -> Rc<Self> {
        let boundary_desc = RefCell::new(vec![DescendantEdges::default(); edges.len()]);
        Rc::new(Self {
            edges,
            boundary_ports: vec![],
            boundary_anc: vec![],
            boundary_desc,
        })
    }

    /// Create a new port diff on `parent` with node `root`
    pub fn with_root(root: V, parent: &Rc<PortDiff<V, P>>) -> Rc<Self> {
        Self::with_nodes([root], parent)
    }

    pub fn with_nodes(nodes: impl IntoIterator<Item = V>, parent: &Rc<PortDiff<V, P>>) -> Rc<Self> {
        let nodes: BTreeSet<_> = nodes.into_iter().collect();
        // Keep edges with both ends in the nodes set
        let edges = parent
            .edges
            .iter()
            .filter(|e| nodes.contains(&e.left.node) && nodes.contains(&e.right.node))
            .cloned()
            .collect_vec();

        let Boundary {
            ports: boundary_ports,
            ancestors: boundary_anc,
        } = compute_boundary(&nodes, parent);
        let boundary_desc = RefCell::new(vec![DescendantEdges::default(); edges.len()]);

        let rc = Rc::new(Self {
            edges,
            boundary_ports,
            boundary_anc,
            boundary_desc,
        });
        // Mark `rc` as a descendant at the ancestors
        record_descendant(&rc);
        rc
    }

    pub fn expand(&self, boundary: BoundaryEdge) -> impl Iterator<Item = Rc<Self>> + '_ {
        let self_nodes: BTreeSet<_> = self.vertices().cloned().collect();
        self.find_opposite_end(boundary)
            .map(move |(opp_owner, opp_edge_end)| {
                // Compute new expanded boundary
                let Boundary {
                    ports: boundary_ports,
                    ancestors: boundary_anc,
                } = {
                    let opp_node = opp_edge_end.node(&opp_owner).clone();
                    let expanded_nodes = {
                        let mut nodes = self_nodes.clone();
                        nodes.insert(opp_node);
                        nodes
                    };
                    compute_boundary(&expanded_nodes, &opp_owner)
                };

                // Add new internal edge
                let edges = {
                    let mut edges = self.edges.clone();
                    let left = self.boundary_edge(&boundary).clone();
                    let right = opp_edge_end.node_port(opp_owner.as_ref()).clone();
                    edges.push(PortEdge { left, right });
                    edges
                };
                // Increase descendants vector size accordingly
                let boundary_desc = {
                    let mut boundary_desc = self.boundary_desc.borrow().clone();
                    boundary_desc.push(DescendantEdges::default());
                    RefCell::new(boundary_desc)
                };
                let rc = Rc::new(Self {
                    edges,
                    boundary_ports,
                    boundary_anc,
                    boundary_desc,
                });
                // Mark `rc` as a descendant at the ancestors
                record_descendant(&rc);
                rc
            })
    }

    pub fn extract(&self) -> Vec<PortEdge<V, P>>
    where
        V: Clone,
        P: Clone,
    {
        if self.boundary_ports.is_empty() {
            return self.edges.clone();
        }
        let mut expanded = Rc::new(self.clone());
        while let Some(boundary) = expanded.boundary_edges().next() {
            let Some(new_expanded) = expanded.expand(boundary).next() else {
                continue;
            };
            expanded = new_expanded;
        }
        expanded.edges.clone()
    }

    /// Traverse a boundary edge and list all possible opposite edge ends
    fn find_opposite_end(
        &self,
        boundary: BoundaryEdge,
    ) -> impl Iterator<Item = (Rc<Self>, EdgeEnd)> {
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
        self.boundary_ports.len()
    }

    fn n_internal_edges(&self) -> usize {
        self.edges.len()
    }

    pub fn boundary_edges(&self) -> impl Iterator<Item = BoundaryEdge> {
        (0..self.n_boundary_edges()).map(BoundaryEdge)
    }

    pub fn boundary_edge(&self, edge: &BoundaryEdge) -> &Port<V, P> {
        let &BoundaryEdge(index) = edge;
        &self.boundary_ports[index]
    }

    pub fn internal_edges(&self) -> impl Iterator<Item = InternalEdge> {
        (0..self.n_internal_edges()).map(InternalEdge)
    }

    pub fn internal_edge(&self, edge: &InternalEdge) -> &PortEdge<V, P> {
        let &InternalEdge(index) = edge;
        &self.edges[index]
    }

    pub fn degree(&self, node: &V) -> usize
    where
        V: Eq,
    {
        self.edges
            .iter()
            .filter(|e| &e.left.node == node || &e.right.node == node)
            .count()
    }

    pub fn vertices(&self) -> impl Iterator<Item = &V>
    where
        V: Eq + Ord,
    {
        self.edges
            .iter()
            .flat_map(|e| [&e.left.node, &e.right.node])
            .chain(self.boundary_ports.iter().map(|p| &p.node))
            .sorted()
            .dedup()
    }

    fn get_ancestor_edge(&self, edge: &BoundaryEdge) -> &AncestorEdge<V, P> {
        let &BoundaryEdge(index) = edge;
        &self.boundary_anc[index]
    }

    pub(crate) fn get_descendant_edges(
        &self,
        edge: &InternalEdge,
        end: EdgeEndType,
    ) -> Ref<[DescendantEdge<V, P>]> {
        let &InternalEdge(index) = edge;
        // Before returning the list, take the opportunity to remove any old
        // weak refs
        self.boundary_desc.borrow_mut()[index].remove_empty_refs();

        let boundary_desc = self.boundary_desc.borrow();
        match end {
            EdgeEndType::Left => Ref::map(boundary_desc, |r| r[index].left.as_slice()),
            EdgeEndType::Right => Ref::map(boundary_desc, |r| r[index].right.as_slice()),
        }
    }

    pub fn has_any_descendants(&self) -> bool {
        self.boundary_desc.borrow().iter().any(|r| !r.is_empty())
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
        let mut desc_map = self.boundary_desc.borrow_mut();
        match end {
            EdgeEndType::Left => desc_map[index].left.push(descendant),
            EdgeEndType::Right => desc_map[index].right.push(descendant),
        }
    }
}

/// Add a weak ref to `desc` at all its ancestors
fn record_descendant<V: Clone + Eq, P>(desc: &Rc<PortDiff<V, P>>) {
    for boundary in desc.boundary_edges() {
        let ancestor_edge = desc.get_ancestor_edge(&boundary);
        let descendant_edge =
            DescendantEdge::new(desc, boundary, ancestor_edge.exclude_vertices().clone());
        ancestor_edge
            .owner()
            .add_descendant(descendant_edge, ancestor_edge.edge_end());
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use rstest::{fixture, rstest};

    use super::*;

    #[fixture]
    fn root_diff() -> Rc<PortDiff<i32, i32>> {
        let ports_0 = (0..3).map(|i| Port { node: 0, port: i }).collect_vec();
        let ports_1 = (0..4).map(|i| Port { node: 1, port: i }).collect_vec();
        let ports_2 = (0..4).map(|i| Port { node: 2, port: i }).collect_vec();
        let ports_3 = (0..3).map(|i| Port { node: 3, port: i }).collect_vec();
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

    #[rstest]
    fn test_port_diff(root_diff: Rc<PortDiff<i32, i32>>) {
        let just_1 = PortDiff::with_root(1, &root_diff);
        assert_eq!(just_1.n_boundary_edges(), 4);
        assert_eq!(just_1.n_internal_edges(), 0);
        let child_1_2 = {
            let edge = just_1.find_boundary_edge(&1, &3).unwrap();
            let expansion_opts = just_1.expand(edge).collect_vec();
            assert_eq!(expansion_opts.len(), 1);
            expansion_opts.into_iter().next().unwrap()
        };
        assert_eq!(child_1_2.n_boundary_edges(), 3 + 3);
        assert_eq!(child_1_2.n_internal_edges(), 1);

        // Check boundary
        let boundary = BTreeSet::from_iter(child_1_2.boundary_ports.clone());
        let exp_boundary = BTreeSet::from_iter(
            (0..3)
                .map(|i| Port { node: 1, port: i })
                .chain((0..3).map(|i| Port { node: 2, port: i })),
        );
        assert_eq!(boundary, exp_boundary);

        // Check internal edges
        assert_eq!(
            child_1_2.edges,
            [PortEdge {
                left: Port { node: 1, port: 3 },
                right: Port { node: 2, port: 3 },
            }]
        );

        dbg!(root_diff);
        dbg!(just_1);
        dbg!(child_1_2);
    }
}
