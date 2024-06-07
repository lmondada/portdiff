use std::{collections::BTreeSet, rc::Rc};

use itertools::{izip, Itertools};

use crate::{
    edges::{AncestorEdge, BoundaryEdge, EdgeEnd, InternalEdge},
    EdgeEndType, Port, PortDiff, PortEdge,
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
pub(super) fn compute_boundary<V: Eq + Ord + Clone, P: Clone>(
    nodes: &BTreeSet<V>,
    parent: &Rc<PortDiff<V, P>>,
) -> Boundary<V, P> {
    // Find the subset of boundary ports of parent
    let boundary_ends = filter_boundary(&nodes, &parent.boundary_ports);
    let internal_ends = filter_internal(&nodes, &parent.edges);
    let all_ends = boundary_ends.chain(internal_ends).collect_vec();

    let mut boundary = Boundary::with_capacity(all_ends.len());
    for edge_end in all_ends {
        let (port, ancestor) = match edge_end {
            EdgeEnd::B(b_edge) => {
                let p = parent.boundary_edge(&b_edge).clone();
                let a = parent.get_ancestor_edge(&b_edge).clone();
                (p, a)
            }
            EdgeEnd::I(i_edge, EdgeEndType::Left) => {
                let p = parent.internal_edge(&i_edge).left.clone();
                let a = AncestorEdge::new_internal_edge(edge_end, parent);
                (p, a)
            }
            EdgeEnd::I(i_edge, EdgeEndType::Right) => {
                let p = parent.internal_edge(&i_edge).right.clone();
                let a = AncestorEdge::new_internal_edge(edge_end, parent);
                (p, a)
            }
        };
        boundary.add(port, ancestor);
    }
    boundary
}

pub(super) struct Boundary<V, P> {
    pub(super) ports: Vec<Port<V, P>>,
    pub(super) ancestors: Vec<AncestorEdge<V, P>>,
}

impl<V, P> Boundary<V, P> {
    fn with_capacity(size: usize) -> Self {
        Self {
            ports: Vec::with_capacity(size),
            ancestors: Vec::with_capacity(size),
        }
    }

    fn add(&mut self, port: Port<V, P>, ancestor: AncestorEdge<V, P>) {
        self.ports.push(port);
        self.ancestors.push(ancestor);
    }
}
