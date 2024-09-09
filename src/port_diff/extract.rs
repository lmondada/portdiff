use crate::{graph_view::MergeStrategy, PortDiffGraph};

use crate::{Graph, PortDiff};

#[derive(Debug)]
pub struct IncompatiblePortDiff;

impl<G: Graph> PortDiff<G> {
    pub fn are_compatible<'a>(diffs: impl IntoIterator<Item = &'a PortDiff<G>>) -> bool
    where
        G: 'a,
    {
        Self::try_merge(diffs.into_iter().map(|d| d.clone())).is_ok()
    }

    pub fn try_merge(
        diffs: impl IntoIterator<Item = PortDiff<G>>,
    ) -> Result<PortDiffGraph<G>, IncompatiblePortDiff> {
        let mut graphs = diffs
            .into_iter()
            .map(|d| PortDiffGraph::from_sinks(vec![d]));

        // cannot use reduce here as we need to handle error
        let Some(fst) = graphs.next() else {
            return Ok(PortDiffGraph::default());
        };

        graphs.try_fold(fst, |mut g1, g2| {
            g1.merge(g2, MergeStrategy::FailOnConflicts)?;
            Ok(g1)
        })
    }

    pub fn extract_graph(diffs: Vec<PortDiff<G>>) -> Result<G, IncompatiblePortDiff> {
        let graph = Self::try_merge(diffs)?;
        let diff = PortDiff::squash(&graph);
        Ok(diff.try_unwrap_graph().unwrap())
    }
}
#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use portgraph::render::DotFormat;
    use portgraph::PortView;
    use rstest::rstest;

    use crate::port_diff::tests::TestPortDiff;

    use super::super::tests::{parent_child_diffs, parent_two_children_diffs};
    use super::*;

    #[test]
    fn test_compatible_empty() {
        let diffs: Vec<TestPortDiff> = vec![];
        assert!(PortDiff::are_compatible(&diffs));
    }

    #[rstest]
    fn test_is_compatible(parent_child_diffs: [TestPortDiff; 2]) {
        let [root_diff, _] = parent_child_diffs;
        let (n0, n1, n2, n3) = root_diff.nodes().collect_tuple().unwrap();
        let child_a = root_diff.identity_subgraph([n0, n1]);
        let child_aa = root_diff.identity_subgraph([n2, n3]);
        assert!(PortDiff::are_compatible(&[child_a, child_aa]));
    }

    #[rstest]
    fn test_is_not_compatible(parent_child_diffs: [TestPortDiff; 2]) {
        let [parent, _] = parent_child_diffs;
        let (n0, n1, n2, n3) = parent.nodes().collect_tuple().unwrap();
        let child_a = parent.identity_subgraph([n0, n1]);
        let child_b = parent.identity_subgraph([n1, n2, n3]);
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

    #[ignore = "TODO this is currently not deterministic"]
    #[rstest]
    fn extract_parent_two_children(parent_two_children_diffs: [TestPortDiff; 3]) {
        let [_, child_1, child_2] = parent_two_children_diffs;
        let graph = PortDiff::extract_graph(vec![child_1, child_2]).unwrap();
        assert_eq!(graph.node_count(), 2);
        insta::assert_snapshot!(graph.dot_string());
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
