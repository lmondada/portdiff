use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use itertools::repeat_n;

use crate::{
    edges::{DescendantEdges, EdgeEnd},
    PortEdge,
};

use super::{
    boundary::{compute_boundary, Boundary},
    PortDiff, PortDiffData, PortDiffPtr,
};

impl<V: Eq + Clone + Ord, P: Clone + Eq> PortDiff<V, P> {
    pub fn merge(&self, other: &Self) -> Option<Self> {
        if !self.is_compatible(other) {
            return None;
        }

        // Merge nodes
        let nodes = self.vertices().chain(other.vertices()).cloned().collect();

        // Compute new expanded boundary
        let Boundary {
            ports: boundary_ports,
            ancestors: boundary_anc,
        } = compute_boundary(&nodes, &[self, other]);

        // Merge edges
        let (mut edges, mut boundary_desc) = merge_edges(self, other);

        // Add new internal edges resulting from the merge
        let new_edges = edges_between(self, other);
        let n_new_edges = new_edges.len();
        edges.extend(new_edges);
        boundary_desc.extend(repeat_n(Default::default(), n_new_edges));

        Some(Self::new(PortDiffData {
            edges,
            boundary_ports,
            boundary_anc,
            boundary_desc: RefCell::new(boundary_desc),
        }))
    }
}

/// Merge internal edges of `diff1` and `diff2`.
///
/// TODO: This assumes that diff1 and diff2 are compatible: `used_vertices` is the
/// (disjoint) union of the `used_vertices` of the boundary edges of `diff1`
/// and `diff2`.
///
/// We use the fact that node IDs are immutable: thus if an edge exists both
/// in `diff1` and `diff2`, it must be that the edge endvertices have not been
/// changed (no new incident edges), and thus the edge must be the same (despite
/// the fact that we would allow duplicates).
fn merge_edges<V: Eq + Clone + Ord, P: Clone + Eq>(
    diff1: &PortDiff<V, P>,
    diff2: &PortDiff<V, P>,
) -> (Vec<PortEdge<V, P>>, Vec<DescendantEdges<V, P>>) {
    let mut edges = diff1.data.edges.clone();
    let mut boundary_desc = diff1.data.boundary_desc.borrow().clone();
    let other_edges = &diff2.data.edges;
    let other_desc = diff2.data.boundary_desc.borrow().clone();
    for (other_edge, other_desc) in other_edges.iter().zip(other_desc) {
        if let Some(index) = edges.iter().position(|e| e == other_edge) {
            boundary_desc[index].append(other_desc);
        } else {
            edges.push(other_edge.clone());
            boundary_desc.push(other_desc);
        }
    }
    (edges, boundary_desc)
}

/// Find boundary edges between `diff1` and `diff2`.
fn edges_between<V: Eq + Clone + Ord, P: Clone>(
    diff1: &PortDiff<V, P>,
    diff2: &PortDiff<V, P>,
) -> Vec<PortEdge<V, P>> {
    diff1
        .boundary_edges()
        .filter_map(|b_edge| {
            diff1
                .find_opposite_end(b_edge)
                .find(|(opp_owner, _)| opp_owner == diff2)
                .and_then(|(_, opp_b_edge)| {
                    if let EdgeEnd::B(opp_b_edge) = opp_b_edge {
                        Some((b_edge, opp_b_edge))
                    } else {
                        // Ignore internal edges, these are being added anyways
                        None
                    }
                })
        })
        .map(|(b_edge, opp_edge)| {
            let left = diff1.boundary_edge(&b_edge).clone();
            let right = diff2.boundary_edge(&opp_edge).clone();
            PortEdge { left, right }
        })
        .collect()
}

impl<V: Eq + Clone + Ord, P: Clone> PortDiff<V, P> {
    /// Checks if two port diffs can be merged.
    ///
    /// Done by checking all common ancestors and making sure that their
    /// respective `exclude_vertices` sets are disjoint.
    pub fn is_compatible(&self, other: &Self) -> bool {
        let self_ancestors = self.get_ancestors(|_| false);
        let other_ancestors = other.get_ancestors(|diff| self_ancestors.contains_key(&diff));
        for (key, self_exclude_vertices) in &self_ancestors {
            if let Some(other_exclude_vertices) = other_ancestors.get(key) {
                if !self_exclude_vertices.is_disjoint(other_exclude_vertices) {
                    return false;
                }
            }
        }
        true
    }

    fn get_ancestors(
        &self,
        stop_here: impl Fn(PortDiffPtr<V, P>) -> bool,
    ) -> AncestorExcludeVertices<V, P> {
        let mut ancs = AncestorExcludeVertices::from_iter([(self.as_ptr(), BTreeSet::new())]);
        let mut curr_nodes = vec![self];
        while let Some(diff) = curr_nodes.pop() {
            let diff_ptr = diff.as_ptr();
            if !stop_here(diff_ptr) {
                for edge in diff.boundary_edges().map(|b| diff.get_ancestor_edge(&b)) {
                    let new_anc = edge.owner();
                    let mut new_exclude_vertices = ancs[&diff_ptr].clone();
                    new_exclude_vertices.append(&mut edge.used_vertices().clone());
                    curr_nodes.push(&new_anc);
                    ancs.insert(new_anc.as_ptr(), new_exclude_vertices);
                }
            }
        }
        ancs
    }
}

type AncestorExcludeVertices<V, P> = BTreeMap<PortDiffPtr<V, P>, BTreeSet<V>>;

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use rstest::rstest;

    use crate::port_diff::tests::{test_nodes, TestPortDiff};

    use super::super::tests::root_diff;
    use super::*;

    #[rstest]
    fn test_get_ancestors(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_aa = PortDiff::with_nodes([nodes[0]], &child_a);

        let all_ancs = child_aa.get_ancestors(|_| false);
        assert_eq!(
            all_ancs.keys().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from_iter([root_diff.as_ptr(), child_a.as_ptr(), child_aa.as_ptr()])
        );

        let child_b = PortDiff::with_nodes([nodes[2], nodes[3]], &root_diff);
        let all_ancs = child_b.get_ancestors(|_| false);
        assert_eq!(
            all_ancs.keys().copied().collect::<BTreeSet<_>>(),
            BTreeSet::from_iter([root_diff.as_ptr(), child_b.as_ptr()])
        );
    }

    #[rstest]
    fn test_is_compatible(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_b = PortDiff::with_nodes([nodes[2], nodes[3]], &root_diff);
        assert!(child_a.is_compatible(&child_b));
    }

    #[rstest]
    fn test_is_not_compatible(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_b = PortDiff::with_nodes([nodes[1], nodes[2], nodes[3]], &root_diff);
        let child_a = child_a
            .rewrite(&[], vec![None; child_a.n_boundary_edges()])
            .unwrap();
        let child_b = child_b
            .rewrite(&[], vec![None; child_b.n_boundary_edges()])
            .unwrap();
        assert!(!child_a.is_compatible(&child_b));
    }

    #[rstest]
    fn test_merge(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_b = PortDiff::with_nodes([nodes[2], nodes[3]], &root_diff);
        let child_a = child_a
            .rewrite(&[], vec![None; child_a.n_boundary_edges()])
            .unwrap();
        let child_b = child_b
            .rewrite(&[], vec![None; child_b.n_boundary_edges()])
            .unwrap();
        let merged = child_a.merge(&child_b).unwrap();
        assert_eq!(merged.n_boundary_edges(), 0);
        assert_eq!(merged.n_internal_edges(), 1)
    }

    #[rstest]
    fn test_merge_with_ancestor(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_a = child_a
            .rewrite(&[], vec![None; child_a.n_boundary_edges()])
            .unwrap();
        let merged = child_a.merge(&root_diff).unwrap();
        assert_eq!(merged.n_boundary_edges(), 0);
        assert_eq!(merged.n_internal_edges(), 4)
    }
}
