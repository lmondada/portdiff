mod rewrite;

use std::{
    cell::{Ref, RefCell},
    rc::Rc,
};

use crate::edges::{
    AncestorEdge, BoundaryEdge, DescendantEdge, DescendantEdges, EdgeEnd, InternalEdge,
};
use itertools::Itertools;

use crate::{EdgeEndType, Port, PortEdge};

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
        // Only edges: self-loops on root
        let edges = parent
            .edges
            .iter()
            .filter(|e| e.left.node == root && e.right.node == root)
            .cloned()
            .collect_vec();
        let boundary_indices = get_incident_edges(root, parent);

        let (boundary_ports, boundary_anc) = create_boundary_arrays(boundary_indices, parent);
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
        let mut new_diff = self.clone();
        new_diff.remove_boundary(boundary);
        self.find_opposite_end(boundary)
            .map(move |(opp_owner, opp_edge_end)| {
                let mut new_diff = new_diff.clone();
                // Compute boundary at `opp_edge`
                let opp_node = opp_edge_end.node(&opp_owner).clone();
                let opp_boundary = get_incident_edges(opp_node, &opp_owner)
                    .into_iter()
                    .filter(|e| e != &opp_edge_end);
                let (boundary_ports, boundary_anc) =
                    create_boundary_arrays(opp_boundary, &opp_owner);
                // Add boundary to new_diff
                new_diff.boundary_ports.extend(boundary_ports);
                new_diff.boundary_anc.extend(boundary_anc);
                // Add new internal edge to new_diff
                let left = self.boundary_edge(&boundary).clone();
                let right = opp_edge_end.node_port(opp_owner.as_ref()).clone();
                new_diff.edges.push(PortEdge { left, right });
                new_diff
                    .boundary_desc
                    .borrow_mut()
                    .push(DescendantEdges::default());
                let rc = Rc::new(new_diff);
                // Mark `rc` as a descendant at the ancestors
                record_descendant(&rc);
                rc
            })
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
    fn n_boundary_edges(&self) -> usize {
        self.boundary_ports.len()
    }

    #[cfg(test)]
    fn n_internal_edges(&self) -> usize {
        self.edges.len()
    }

    fn boundary_edges(&self) -> impl Iterator<Item = BoundaryEdge> {
        (0..self.n_boundary_edges()).map(BoundaryEdge)
    }

    pub fn boundary_edge(&self, edge: &BoundaryEdge) -> &Port<V, P> {
        let &BoundaryEdge(index) = edge;
        &self.boundary_ports[index]
    }

    pub fn internal_edge(&self, edge: &InternalEdge) -> &PortEdge<V, P> {
        let &InternalEdge(index) = edge;
        &self.edges[index]
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

    fn remove_boundary(&mut self, boundary: BoundaryEdge) {
        let &BoundaryEdge(index) = &boundary;
        self.boundary_ports.remove(index);
        self.boundary_anc.remove(index);
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

    /// TODO: Actually extract a valid graph with boundaries
    pub fn extract(&self) -> Vec<PortEdge<V, P>>
    where
        V: Clone,
        P: Clone,
    {
        if self.boundary_ports.is_empty() {
            return self.edges.clone();
        }
        unimplemented!()
    }
}

/// The list of edges (either internal or boundary) adjacent to root in parent.
///
/// TODO: Sorted for consistency across diffs.
fn get_incident_edges<V: Eq, P>(root: V, parent: impl AsRef<PortDiff<V, P>>) -> Vec<EdgeEnd> {
    let parent = parent.as_ref();
    let from_boundary = parent
        .boundary_ports
        .iter()
        .enumerate()
        .filter(|(_, p)| p.node == root)
        .map(|(i, _)| EdgeEnd::B(BoundaryEdge(i)));
    let from_internal = parent.edges.iter().enumerate().filter_map(|(i, e)| {
        let is_left_end = e.left.node == root;
        let is_right_end = e.right.node == root;
        match (is_left_end, is_right_end) {
            (true, true) | (false, false) => None,
            (true, false) => Some(EdgeEnd::I(InternalEdge(i), EdgeEndType::Left)),
            (false, true) => Some(EdgeEnd::I(InternalEdge(i), EdgeEndType::Right)),
        }
    });
    let boundary = from_boundary.chain(from_internal).collect();
    boundary
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

fn create_boundary_arrays<V: Clone, P: Clone>(
    boundary: impl IntoIterator<Item = EdgeEnd>,
    diff: &Rc<PortDiff<V, P>>,
) -> (Vec<Port<V, P>>, Vec<AncestorEdge<V, P>>) {
    let mut boundary_ports = Vec::new();
    let mut boundary_anc = Vec::new();
    for ind in boundary {
        match ind {
            EdgeEnd::B(b_edge) => {
                boundary_ports.push(diff.boundary_edge(&b_edge).clone());
                boundary_anc.push(diff.get_ancestor_edge(&b_edge).clone());
            }
            edge_end @ EdgeEnd::I(i_edge, EdgeEndType::Left) => {
                boundary_ports.push(diff.internal_edge(&i_edge).left.clone());
                let ancestor = AncestorEdge::new_internal_edge(edge_end, diff);
                boundary_anc.push(ancestor);
            }
            edge_end @ EdgeEnd::I(i_edge, EdgeEndType::Right) => {
                boundary_ports.push(diff.internal_edge(&i_edge).right.clone());
                let ancestor = AncestorEdge::new_internal_edge(edge_end, diff);
                boundary_anc.push(ancestor);
            }
        }
    }
    (boundary_ports, boundary_anc)
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
