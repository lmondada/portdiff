use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use itertools::izip;

use crate::{
    edges::{BoundaryEdge, EdgeEnd, InternalEdge},
    EdgeEndType, PortEdge,
};

use super::{PortDiff, PortDiffData, PortDiffPtr};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeType {
    DisjointMerge,
    AncestorMerge,
    ReverseAncestorMerge,
    NoMerge,
}

impl<V: Eq + Clone + Ord, P: Clone + Eq> PortDiff<V, P> {
    pub fn merge(&self, other: &Self) -> Option<Self> {
        Some(PortDiff::new(self.merge_data(other)?))
    }

    pub fn merge_all<'d>(
        diffs: impl IntoIterator<Item = &'d Self>,
    ) -> Result<PortDiff<V, P>, String>
    where
        P: 'd,
        V: 'd,
    {
        let mut merged_diff: Option<PortDiff<V, P>> = None;
        for diff in diffs {
            merged_diff = if let Some(merged_diff) = merged_diff {
                merged_diff.merge(&diff)
            } else {
                Some(diff.clone())
            };
            if merged_diff.is_none() {
                return Err("Cannot merge diffs".to_string());
            }
        }
        merged_diff.ok_or("Cannot merge empty diff set".to_string())
    }

    fn merge_data(&self, other: &Self) -> Option<PortDiffData<V, P>> {
        match self.merge_type(other) {
            MergeType::DisjointMerge => Some(self.merge_disjoint(other)),
            MergeType::AncestorMerge => Some(self.merge_ancestor(other)),
            MergeType::ReverseAncestorMerge => Some(other.merge_ancestor(self)),
            MergeType::NoMerge => None,
        }
    }

    /// Merge two disjoint port diffs.
    ///
    /// Does not check for disjoint-ness, use `merge` for that.
    fn merge_disjoint(&self, other: &Self) -> PortDiffData<V, P> {
        let mut edges = self.data.edges.clone();
        edges.extend(other.data.edges.clone());

        let mut boundary_ports = self.data.boundary_ports.clone();
        boundary_ports.extend(other.data.boundary_ports.clone());

        let mut boundary_anc = self.data.boundary_anc.clone();
        boundary_anc.extend(other.data.boundary_anc.clone());

        let mut boundary_desc = self.data.boundary_desc.borrow().clone();
        boundary_desc.extend(other.data.boundary_desc.borrow().clone());

        PortDiffData {
            edges,
            boundary_ports,
            boundary_anc,
            boundary_desc: RefCell::new(boundary_desc),
        }
    }

    fn merge_ancestor(&self, ancestor: &Self) -> PortDiffData<V, P> {
        let used_vertices = used_vertices(ancestor, self);

        // The resulting set of nodes are the union of the node sets minus
        // the used nodes on the path.
        let valid_ancestor_nodes: BTreeSet<_> = {
            let nodes: BTreeSet<_> = ancestor.vertices().cloned().collect();
            nodes.difference(&used_vertices).cloned().collect()
        };

        // Edges are the union of
        //  - the edges from `self` and `ancestor` that have both ends in merged_nodes, and
        //  - result from a boundary in `self` to an edge in `ancestor`
        let (valid_ancestor_edges, valid_ancestor_desc): (Vec<_>, Vec<_>) = {
            let all_ancestor_edges = ancestor.data.edges.iter();
            let all_ancestor_desc = ancestor.data.boundary_desc.borrow();

            izip!(all_ancestor_edges, all_ancestor_desc.iter())
                .filter(|(e, _)| {
                    valid_ancestor_nodes.contains(&e.left.node)
                        && valid_ancestor_nodes.contains(&e.right.node)
                })
                .map(|(e, desc)| (e.clone(), desc.clone()))
                .unzip()
        };
        let mut merged_edges = self.data.edges.clone();
        let mut merged_desc = self.data.boundary_desc.borrow().clone();
        merged_edges.extend(valid_ancestor_edges);
        merged_desc.extend(valid_ancestor_desc);

        // Second point that I said above
        let mut viewed_boundaries = BTreeSet::new();
        for (b_edge, i_edge) in self.boundary_edges().zip(self.map_boundaries(ancestor)) {
            if viewed_boundaries.contains(&b_edge) {
                continue;
            }
            if let Some((i_edge, end_type)) = i_edge {
                let left = self.boundary_edge(&b_edge).clone();
                let right = ancestor
                    .port(EdgeEnd::I(i_edge, end_type.opposite()))
                    .clone();
                merged_edges.push(PortEdge { left, right });
                merged_desc.push(Default::default());
                if let Some(desc) = ancestor
                    .get_descendant_edges(&i_edge, end_type.opposite())
                    .iter()
                    .find(|desc| desc.owner().as_ref() == Some(self))
                {
                    viewed_boundaries.insert(desc.boundary_edge());
                }
            }
        }
        PortDiffData {
            edges: merged_edges,
            boundary_ports: ancestor.data.boundary_ports.clone(),
            boundary_anc: ancestor.data.boundary_anc.clone(),
            boundary_desc: RefCell::new(merged_desc),
        }
    }

    /// Map the boundary edges of `self` to edges in `ancestor`.
    ///
    /// The i-th value in the returned vector is the internal edge in `ancestor`
    /// that `BoundaryEdge(i)` of `self` is mapped to, if it exists.
    fn map_boundaries(&self, ancestor: &Self) -> Vec<Option<(InternalEdge, EdgeEndType)>> {
        let mut map = vec![None; self.n_boundary_edges()];
        for b_edge in self.boundary_edges() {
            let anc = self.get_ancestor_edge(&b_edge);
            if anc.owner() == ancestor {
                map[b_edge.0] = Some((anc.internal_edge(), anc.edge_end_type()));
            } else {
                // If `ancestor` is an ancestor of `anc`, then the edge might exist
                // as is in `ancestor`
                let i_edge = anc.owner().internal_edge(&anc.internal_edge());
                if let Some(anc_edge) = ancestor.find_edge(i_edge) {
                    map[b_edge.0] = Some((anc_edge, anc.edge_end_type()));
                }
            }
        }
        map
    }

    pub fn expand(&self, boundary: BoundaryEdge) -> impl Iterator<Item = Self> + '_
    where
        P: Eq,
    {
        self.find_opposite_end(boundary)
            .filter_map(move |(opp_owner, opp_edge_end)| {
                let mut data = if &opp_owner == self {
                    (*self.data).clone()
                } else {
                    self.merge_data(&opp_owner)?
                };
                // If both ends are boundaries, then this is a disjoint merge
                // and we must remove the boundaries from the new diff
                if let EdgeEnd::B(opp_boundary) = opp_edge_end {
                    // Add the new internal edge
                    let left = self.boundary_edge(&boundary).clone();
                    let right = opp_owner.port(opp_edge_end).clone();
                    data.edges.push(PortEdge { left, right });
                    data.boundary_ports.remove(boundary.0 + opp_boundary.0);
                    data.boundary_anc.remove(boundary.0 + opp_boundary.0);
                    data.boundary_ports.remove(boundary.0);
                    data.boundary_anc.remove(boundary.0);
                }
                Some(PortDiff::new(data))
            })
    }
}

/// The vertices used up on the path `from` until `to`
fn used_vertices<V: Eq + Clone + Ord, P: Clone + Eq>(
    from: &PortDiff<V, P>,
    to: &PortDiff<V, P>,
) -> BTreeSet<V> {
    // TODO: Computing this might be a bit wasteful...
    let mut all_ancs = to.get_ancestors(|d| d == from.as_ptr());
    all_ancs.remove(&from.as_ptr()).unwrap()
}

impl<V: Eq + Clone + Ord, P: Clone> PortDiff<V, P> {
    /// Map the boundaries of `other` onto ancestors of `self`.
    pub fn merge_type(&self, other: &Self) -> MergeType {
        let self_ancestors = self.get_ancestors(|_| false);
        let other_ancestors = other.get_ancestors(|_| false);
        if self_ancestors.contains_key(&other.as_ptr()) {
            return MergeType::AncestorMerge;
        } else if other_ancestors.contains_key(&self.as_ptr()) {
            return MergeType::ReverseAncestorMerge;
        }
        let other_ancestors = other.get_ancestors(|diff| self_ancestors.contains_key(&diff));
        for (key, self_used_vertices) in &self_ancestors {
            if let Some(other_used_vertices) = other_ancestors.get(key) {
                if !self_used_vertices.is_disjoint(other_used_vertices) {
                    return MergeType::NoMerge;
                }
            }
        }
        return MergeType::DisjointMerge;
    }

    /// A map from ancestors to the set of used vertices.
    ///
    /// Used vertices are the vertices that are used on the path from the
    /// ancestor to `self`.
    fn get_ancestors(
        &self,
        stop_here: impl Fn(PortDiffPtr<V, P>) -> bool,
    ) -> AncestorUsedVertices<V, P> {
        let mut ancs =
            AncestorUsedVertices::from_iter([(self.as_ptr(), self.vertices().cloned().collect())]);
        let mut curr_nodes = vec![self];
        while let Some(diff) = curr_nodes.pop() {
            let diff_ptr = diff.as_ptr();
            if !stop_here(diff_ptr) {
                for edge in diff.boundary_edges().map(|b| diff.get_ancestor_edge(&b)) {
                    let new_anc = edge.owner();
                    let mut new_used_vertices = ancs[&diff_ptr].clone();
                    new_used_vertices.append(&mut edge.used_vertices().clone());
                    curr_nodes.push(&new_anc);
                    ancs.entry(new_anc.as_ptr())
                        .or_default()
                        .extend(new_used_vertices);
                }
            }
        }
        ancs
    }
}

type AncestorUsedVertices<V, P> = BTreeMap<PortDiffPtr<V, P>, BTreeSet<V>>;

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::port_diff::tests::{test_nodes, TestPortDiff};
    use crate::Port;

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
        assert_eq!(child_a.merge_type(&child_b), MergeType::DisjointMerge);
    }

    #[rstest]
    fn test_is_not_compatible(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_b = PortDiff::with_nodes([nodes[1], nodes[2], nodes[3]], &root_diff);
        let child_a = child_a
            .rewrite(&[], &vec![None; child_a.n_boundary_edges()])
            .unwrap();
        let child_b = child_b
            .rewrite(&[], &vec![None; child_b.n_boundary_edges()])
            .unwrap();
        assert_eq!(child_a.merge_type(&child_b), MergeType::NoMerge);
    }

    #[rstest]
    fn test_merge(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_b = PortDiff::with_nodes([nodes[2], nodes[3]], &root_diff);
        let child_a = child_a
            .rewrite(&[], &vec![None; child_a.n_boundary_edges()])
            .unwrap();
        let child_b = child_b
            .rewrite(&[], &vec![None; child_b.n_boundary_edges()])
            .unwrap();
        let merged = PortDiff::new(child_a.merge_disjoint(&child_b));
        assert_eq!(merged.n_boundary_edges(), 2);
        assert_eq!(merged.n_internal_edges(), 0);
        let merged = merged
            .expand(merged.boundary_edges().next().unwrap())
            .next()
            .unwrap();
        assert_eq!(merged.n_internal_edges(), 1);
        assert_eq!(merged.n_boundary_edges(), 0);
    }

    #[rstest]
    fn test_merge_with_ancestor(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_a = child_a
            .rewrite(&[], &vec![None; child_a.n_boundary_edges()])
            .unwrap();
        let merged = child_a.merge(&root_diff).unwrap();
        assert_eq!(merged.n_boundary_edges(), 0);
        assert_eq!(merged.n_internal_edges(), 4)
    }

    #[rstest]
    fn test_merge_with_ancestor_2(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let child_a = child_a
            .rewrite(
                &[PortEdge {
                    left: Port {
                        node: nodes[0],
                        port: 5,
                    },
                    right: Port {
                        node: nodes[1],
                        port: 3,
                    },
                }],
                &vec![None; child_a.n_boundary_edges()],
            )
            .unwrap();
        let merged = child_a.merge(&root_diff).unwrap();
        assert_eq!(merged.n_boundary_edges(), 0);
        assert_eq!(merged.n_internal_edges(), 5)
    }

    #[rstest]
    fn test_merge_replace_vertex(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child_a = PortDiff::with_nodes([nodes[0], nodes[1], nodes[2]], &root_diff);
        let child_a = child_a
            .rewrite(
                &[
                    PortEdge {
                        left: Port {
                            node: nodes[0],
                            port: 0,
                        },
                        right: Port {
                            node: nodes[1],
                            port: 3,
                        },
                    },
                    PortEdge {
                        left: Port {
                            node: nodes[1],
                            port: 1,
                        },
                        right: Port {
                            node: nodes[2],
                            port: 3,
                        },
                    },
                ],
                &vec![None; child_a.n_boundary_edges()],
            )
            .unwrap();

        let child_b = PortDiff::with_nodes([nodes[3]], &root_diff);
        let mut merged = child_a.merge(&child_b).unwrap();

        assert_eq!(merged.n_internal_edges(), 2);
        assert_eq!(merged.n_boundary_edges(), 6);
        while let Some(b_edge) = merged.boundary_edges().next() {
            let tmp = merged.expand(b_edge).next().unwrap();
            merged = tmp;
        }
        assert_eq!(merged.n_internal_edges(), 5);
        assert_eq!(merged.n_boundary_edges(), 0);
    }
}
