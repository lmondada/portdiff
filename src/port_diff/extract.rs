use std::collections::BTreeSet;

use itertools::Itertools;
use petgraph::visit::{EdgeRef, IntoEdges};
use relrc::AncestorGraph;

use crate::{Graph, PortDiff};

use super::EdgeData;

#[derive(Debug)]
pub struct IncompatiblePortDiff;

impl<G: Graph> PortDiff<G> {
    pub fn are_compatible<'a>(diffs: impl IntoIterator<Item = &'a PortDiff<G>>) -> bool
    where
        G: 'a,
    {
        let graphs = diffs
            .into_iter()
            .map(|d| AncestorGraph::from_terminals(vec![d.data.clone()]))
            .collect_vec();
        let merged_graph = graphs
            .clone()
            .into_iter()
            .reduce(|mut g1, g2| {
                g1.merge(g2);
                g1
            })
            .unwrap();
        // For each node that is an ancestor of two or more graphs...
        for diff_ptr in AncestorGraph::lowest_common_ancestors(&graphs) {
            // Check that its outgoing edges are compatible
            // (this must hold everywhere, but already holds elsewhere as the
            // set of outgoing edges in non-lca nodes remains unchanged).
            let edges = merged_graph.edges(diff_ptr).collect_vec();
            if !EdgeData::are_compatible(edges.iter().map(|e| e.weight())) {
                return false;
            }
        }
        true
    }

    pub fn extract_graph(mut diffs: Vec<PortDiff<G>>) -> Result<G, IncompatiblePortDiff> {
        if !PortDiff::are_compatible(&diffs) {
            return Err(IncompatiblePortDiff);
        }
        // We repeatedly squash the leaves in `diffs`. When the `diff` no longer
        // has a parent we can add it to the output.
        let mut final_diffs = vec![];
        while let Some(diff) = diffs.pop() {
            let others =
                AncestorGraph::from_terminals(diffs.iter().map(|d| d.data.clone()).collect());
            if others
                .all_nodes()
                .contains(&PortDiff::as_ptr(&diff.data.clone().into()).into())
            {
                continue;
            } else if diff.all_parents().next().is_none() {
                final_diffs.push(diff);
            } else {
                diffs.push(diff.squash());
            }
        }
        let mut out_graph = G::default();
        for diff in final_diffs {
            out_graph.add_subgraph(&diff.graph, &BTreeSet::new());
        }
        Ok(out_graph)
    }
}
#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::port_diff::tests::TestPortDiff;

    use super::super::tests::root_diff;
    use super::*;

    #[rstest]
    fn test_is_compatible(root_diff: TestPortDiff) {
        let (n0, n1, n2, n3) = root_diff.nodes().collect_tuple().unwrap();
        let child_a = root_diff.identity_subgraph([n0, n1]);
        let child_aa = root_diff.identity_subgraph([n2, n3]);
        assert!(PortDiff::are_compatible(&[child_a, child_aa]));
    }

    #[rstest]
    fn test_is_not_compatible(root_diff: TestPortDiff) {
        let (n0, n1, n2, n3) = root_diff.nodes().collect_tuple().unwrap();
        let child_a = root_diff.identity_subgraph([n0, n1]);
        let child_b = root_diff.identity_subgraph([n1, n2, n3]);
        assert_eq!(
            child_a
                .incoming(0.into())
                .unwrap()
                .value()
                .subgraph
                .nodes()
                .len(),
            2
        );
        assert_eq!(
            child_b
                .incoming(0.into())
                .unwrap()
                .value()
                .subgraph
                .nodes()
                .len(),
            3
        );
        assert!(!PortDiff::are_compatible(&[child_a, child_b]));
    }

    // #[rstest]
    // fn test_merge(root_diff: TestPortDiff) {
    //     let nodes = test_nodes();
    //     let mut vertex_creator = DetVertexCreator { max_ind: 4 };

    //     let child_a = PortDiff::with_nodes([nodes[0].clone(), nodes[1].clone()], &root_diff);
    //     let child_b = PortDiff::with_nodes([nodes[2].clone(), nodes[3].clone()], &root_diff);
    //     let child_a = child_a
    //         .rewrite(
    //             &[],
    //             &vec![None; child_a.n_boundary_edges()],
    //             &mut vertex_creator,
    //         )
    //         .unwrap();
    //     let child_b = child_b
    //         .rewrite(
    //             &[],
    //             &vec![None; child_b.n_boundary_edges()],
    //             &mut vertex_creator,
    //         )
    //         .unwrap();
    //     let merged = PortDiff::new(child_a.merge_disjoint(&child_b));
    //     assert_eq!(merged.n_boundary_ports(), 2);
    //     assert_eq!(merged.n_internal_edges(), 0);
    //     let merged = merged
    //         .expand(merged.boundary_edges().next().unwrap())
    //         .next()
    //         .unwrap();
    //     assert_eq!(merged.n_internal_edges(), 1);
    //     assert_eq!(merged.n_boundary_edges(), 0);
    // }

    // #[rstest]
    // fn test_merge_with_ancestor(root_diff: TestPortDiff) {
    //     let nodes = test_nodes();
    //     let mut vertex_creator = DetVertexCreator { max_ind: 4 };

    //     let child_a = PortDiff::with_nodes([nodes[0].clone(), nodes[1].clone()], &root_diff);
    //     let child_a = child_a
    //         .rewrite(
    //             &[],
    //             &vec![None; child_a.n_boundary_edges()],
    //             &mut vertex_creator,
    //         )
    //         .unwrap();
    //     let merged = child_a.merge(&root_diff).unwrap();
    //     assert_eq!(merged.n_boundary_edges(), 0);
    //     assert_eq!(merged.n_internal_edges(), 4)
    // }

    // #[rstest]
    // fn test_merge_with_ancestor_2(root_diff: TestPortDiff) {
    //     let nodes = test_nodes();
    //     let mut vertex_creator = DetVertexCreator { max_ind: 4 };

    //     let child_a = PortDiff::with_nodes([nodes[0].clone(), nodes[1].clone()], &root_diff);
    //     let child_a = child_a
    //         .rewrite(
    //             &[PortEdge {
    //                 left: UnboundPort {
    //                     node: nodes[0].clone(),
    //                     port: 5,
    //                 },
    //                 right: UnboundPort {
    //                     node: nodes[1].clone(),
    //                     port: 3,
    //                 },
    //             }],
    //             &vec![None; child_a.n_boundary_edges()],
    //             &mut vertex_creator,
    //         )
    //         .unwrap();
    //     let merged = child_a.merge(&root_diff).unwrap();
    //     assert_eq!(merged.n_boundary_edges(), 0);
    //     assert_eq!(merged.n_internal_edges(), 5)
    // }

    // #[rstest]
    // fn test_merge_replace_vertex(root_diff: TestPortDiff) {
    //     let nodes = test_nodes();
    //     let mut vertex_creator = DetVertexCreator { max_ind: 4 };

    //     let child_a = PortDiff::with_nodes(
    //         [nodes[0].clone(), nodes[1].clone(), nodes[2].clone()],
    //         &root_diff,
    //     );
    //     let child_a = child_a
    //         .rewrite(
    //             &[
    //                 PortEdge {
    //                     left: UnboundPort {
    //                         node: nodes[0].clone(),
    //                         port: 0,
    //                     },
    //                     right: UnboundPort {
    //                         node: nodes[1].clone(),
    //                         port: 3,
    //                     },
    //                 },
    //                 PortEdge {
    //                     left: UnboundPort {
    //                         node: nodes[1].clone(),
    //                         port: 1,
    //                     },
    //                     right: UnboundPort {
    //                         node: nodes[2].clone(),
    //                         port: 3,
    //                     },
    //                 },
    //             ],
    //             &vec![None; child_a.n_boundary_edges()],
    //             &mut vertex_creator,
    //         )
    //         .unwrap();

    //     let child_b = PortDiff::with_nodes([nodes[3].clone()], &root_diff);
    //     let mut merged = child_a.merge(&child_b).unwrap();

    //     assert_eq!(merged.n_internal_edges(), 2);
    //     assert_eq!(merged.n_boundary_edges(), 6);
    //     while let Some(b_edge) = merged.boundary_edges().next() {
    //         let tmp = merged.expand(b_edge).next().unwrap();
    //         merged = tmp;
    //     }
    //     assert_eq!(merged.n_internal_edges(), 5);
    //     assert_eq!(merged.n_boundary_edges(), 0);
    // }
}
