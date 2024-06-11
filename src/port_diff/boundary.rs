use std::collections::BTreeSet;

use itertools::{izip, Itertools};

use crate::{
    edges::{AncestorEdge, BoundaryEdge, EdgeEnd, InternalEdge},
    Port, PortDiff, PortEdge,
};

fn gen_boundary_ends() -> impl Iterator<Item = EdgeEnd> {
    (0..).map(|i| EdgeEnd::B(BoundaryEdge(i)))
}

fn gen_internal_edges() -> impl Iterator<Item = InternalEdge> {
    (0..).map(InternalEdge)
}

fn filter_boundary<'p, V: Eq + Ord + 'p, P: 'p>(
    nodes: &'p BTreeSet<V>,
    ports: &'p [Port<V, P>],
) -> impl Iterator<Item = EdgeEnd> + 'p {
    izip!(ports, gen_boundary_ends()).filter_map(|(p, e)| nodes.contains(&p.node).then_some(e))
}

fn filter_internal<'p, V: Eq + Ord + 'p, P: 'p>(
    nodes: &'p BTreeSet<V>,
    edges: &'p [PortEdge<V, P>],
) -> impl Iterator<Item = EdgeEnd> + 'p {
    izip!(edges, gen_internal_edges()).filter_map(|(e, edge_ind)| {
        let is_left_contained = nodes.contains(&e.left.node);
        let is_right_contained = nodes.contains(&e.right.node);
        match (is_left_contained, is_right_contained) {
            (true, true) | (false, false) => None,
            (true, false) => Some(edge_ind.to_left_end()),
            (false, true) => Some(edge_ind.to_right_end()),
        }
    })
}

/// The list of edges (either internal or boundary) adjacent to root in parent.
///
/// TODO: Sorted for consistency across diffs.
pub(super) fn compute_boundary<'a, V: Eq + Ord + Clone, P: Clone>(
    nodes: &BTreeSet<V>,
    parents: &[&'a PortDiff<V, P>],
) -> Boundary<'a, V, P> {
    // Find the subset of boundary ports of parent
    let boundary_ends = parents.iter().flat_map(|parent| {
        filter_boundary(&nodes, &parent.data.boundary_ports).map(move |e| (e, parent))
    });
    // Remove boundary ends that lead to another `parent`
    let boundary_ends = boundary_ends.filter(|(e, parent)| {
        let &EdgeEnd::B(b_edge) = e else {
            unreachable!()
        };
        !parent
            .find_opposite_end(b_edge)
            .any(|(p, _)| parents.contains(&&p))
    });
    let internal_ends = parents
        .iter()
        .flat_map(|parent| filter_internal(&nodes, &parent.data.edges).map(move |e| (e, parent)));
    let all_ends = boundary_ends.chain(internal_ends).collect_vec();

    let mut boundary = Boundary::with_capacity(all_ends.len());
    for (edge_end, parent) in all_ends {
        let ancestor = match edge_end {
            EdgeEnd::B(b_edge) => parent.get_ancestor_edge(&b_edge).clone(),
            EdgeEnd::I(_, _) => AncestorEdge::new_internal_edge(edge_end, parent),
        };
        boundary.add(edge_end, parent, ancestor);
    }
    boundary
}

pub(super) struct Boundary<'a, V, P> {
    edge_ends: Vec<EdgeEnd>,
    owners: Vec<&'a PortDiff<V, P>>,
    ancestors: Vec<AncestorEdge<V, P>>,
}

impl<'a, V, P> Boundary<'a, V, P> {
    fn with_capacity(size: usize) -> Self {
        Self {
            edge_ends: Vec::with_capacity(size),
            owners: Vec::with_capacity(size),
            ancestors: Vec::with_capacity(size),
        }
    }

    pub(super) fn ports(&self) -> impl Iterator<Item = &Port<V, P>> + '_ {
        self.owners
            .iter()
            .zip(&self.edge_ends)
            .map(|(owner, edge_end)| owner.port(*edge_end))
    }

    pub(super) fn into_ancestors(self) -> Vec<AncestorEdge<V, P>> {
        self.ancestors
    }

    fn add(&mut self, edge_end: EdgeEnd, owner: &'a PortDiff<V, P>, ancestor: AncestorEdge<V, P>) {
        self.edge_ends.push(edge_end);
        self.owners.push(owner);
        self.ancestors.push(ancestor);
    }
}
