use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;
use petgraph::{algo, dot::Dot, graph::DiGraph, Direction};

use crate::{
    graph::GraphBuilder,
    port::{BoundPort, ParentPort, Port, PortSide, UnboundPort},
    Graph, PortDiff,
};

use super::UniqueNodeId;

#[derive(Debug)]
pub struct IncompatiblePortDiff;

impl<G: Graph> PortDiff<G> {
    pub fn are_compatible<'a>(diffs: impl IntoIterator<Item = &'a PortDiff<G>>) -> bool
    where
        G: 'a,
    {
        let graph = hierarchy_graph(diffs);

        for diff_node in algo::toposort(&graph, None).unwrap() {
            let diff = graph[diff_node];
            let mut exclude_nodes = BTreeSet::new();
            for prev in graph.neighbors_directed(diff_node, Direction::Incoming) {
                let prev_diff = graph[prev];
                if !prev_diff
                    .exclude_nodes(diff)
                    .all(|n| exclude_nodes.insert(n))
                {
                    return false;
                }
            }
        }
        true
    }

    pub fn extract_graph<Out: GraphBuilder<G>>(
        diffs: &[PortDiff<G>],
    ) -> Result<Out, IncompatiblePortDiff> {
        let mut out_graph = Out::new();
        let mut boundary = ExtractionBoundary::new();

        let graph = hierarchy_graph(diffs);

        for diff_node in algo::toposort(&graph, None).unwrap() {
            let diff = graph[diff_node];
            let mut exclude_nodes = BTreeSet::new();
            for prev in graph.neighbors_directed(diff_node, Direction::Incoming) {
                let prev_diff = graph[prev];
                if !prev_diff
                    .exclude_nodes(diff)
                    .all(|n| exclude_nodes.insert(n))
                {
                    return Err(IncompatiblePortDiff);
                }
            }
            let remaining_nodes = diff
                .graph()
                .nodes_iter()
                .filter(|n| !exclude_nodes.contains(n));
            let mut ports = boundary.drain_ports(diff);
            let exclude_edges = ports.keys().map(|p| p.edge);
            let node_map = out_graph.add_subgraph(diff.graph(), remaining_nodes, exclude_edges);
            boundary.node_map.extend(
                node_map
                    .into_iter()
                    .map(|(k, v)| (UniqueNodeId::new(k, diff.clone()), v)),
            );

            while let Some((bound_port, unbound_port)) = ports.pop_first() {
                // Find the opposite port, either in `diff` or in the extraction boundary
                let opp_bound_port = bound_port.opposite();
                let opp_node = diff.graph().to_unbound(opp_bound_port).node;
                let opp_unbound_port = if !exclude_nodes.contains(&opp_node) {
                    // Add an edge between a boundary port and `diff`
                    let opp_port = diff.graph().to_unbound(opp_bound_port);
                    boundary.remap_port(opp_port, diff).unwrap()
                } else {
                    // Add an edge between two boundary ports
                    let children = diff.children(opp_bound_port);
                    let Some(child_opp_port) = children.iter().find_map(|child| {
                        let Port::Unbound { port, owner } = child.upgrade()? else {
                            unreachable!("upgrade always returns unbound port")
                        };
                        let opp_port = boundary.remap_port(port, &owner)?;
                        ports
                            .iter()
                            .find_map(|(k, v)| (v == &opp_port).then_some(k))
                            .copied()
                    }) else {
                        panic!("No matching opposite port found in extract_graph");
                    };
                    ports.remove(&child_opp_port).unwrap()
                };

                // Add edge
                match bound_port.port {
                    PortSide::Left => {
                        out_graph.add_edge(unbound_port, opp_unbound_port);
                    }
                    PortSide::Right => {
                        out_graph.add_edge(opp_unbound_port, unbound_port);
                    }
                }
            }

            // Add the ports of `diff` to the boundary
            boundary.add_ports(
                diff.boundary
                    .keys()
                    .filter(|port| !exclude_nodes.contains(&port.node))
                    .cloned(),
                diff,
            );
        }

        Ok(out_graph)
    }
}

fn hierarchy_graph<'a, G: Graph + 'a>(
    diffs: impl IntoIterator<Item = &'a PortDiff<G>>,
) -> DiGraph<&'a PortDiff<G>, ()> {
    // Use petgraph to toposort the hierarchy of diffs
    // First build petgraph (edges pointing from children to parents)
    let mut visit_stack = diffs.into_iter().collect_vec();
    let mut visited = BTreeSet::new();
    let mut diff_to_node = BTreeMap::new();
    let mut graph = DiGraph::new();
    while let Some(diff) = visit_stack.pop() {
        if !visited.insert(diff.as_ptr()) {
            continue;
        }
        let curr_node = *diff_to_node
            .entry(diff.as_ptr())
            .or_insert_with(|| graph.add_node(diff));
        for parent in diff.parents() {
            let parent_node = *diff_to_node
                .entry(parent.as_ptr())
                .or_insert_with(|| graph.add_node(parent));
            graph.add_edge(curr_node, parent_node, ());
            visit_stack.push(parent);
        }
    }
    graph
}

struct ExtractionBoundary<G: Graph, N: Ord + Copy> {
    boundary: Vec<(PortDiff<G>, UnboundPort<G::Node, G::PortLabel>)>,
    node_map: BTreeMap<UniqueNodeId<G>, N>,
}

impl<G: Graph, N: Ord + Copy> ExtractionBoundary<G, N> {
    fn new() -> Self {
        Self {
            boundary: Vec::new(),
            node_map: BTreeMap::new(),
        }
    }

    /// Remove ports from `self` whose parent is `diff`.
    ///
    /// Return a map from bound ports in `diff` to unbound ports in the extraction
    /// boundary.
    fn drain_ports<'a>(
        &'a mut self,
        diff: &'a PortDiff<G>,
    ) -> BTreeMap<BoundPort<G::Edge>, UnboundPort<N, G::PortLabel>> {
        let Self { boundary, node_map } = self;
        let mut map = BTreeMap::new();
        boundary.retain(|(owner, port)| {
            let port_node = node_map[&UniqueNodeId::new(port.node, owner.clone())];
            let boundary_port = UnboundPort {
                node: port_node,
                port: port.port.clone(),
            };
            let ParentPort { parent, port } = owner.parent_port(port);
            if parent == diff {
                map.insert(*port, boundary_port);
                false
            } else {
                true
            }
        });
        map
    }

    fn remap_port(
        &self,
        port: UnboundPort<G::Node, G::PortLabel>,
        owner: &PortDiff<G>,
    ) -> Option<UnboundPort<N, G::PortLabel>> {
        let node = *self
            .node_map
            .get(&UniqueNodeId::new(port.node, owner.clone()))?;
        Some(UnboundPort {
            node,
            port: port.port,
        })
    }

    fn add_ports(
        &mut self,
        ports: impl IntoIterator<Item = UnboundPort<G::Node, G::PortLabel>>,
        diff: &PortDiff<G>,
    ) {
        let known_parents: BTreeSet<_> = self
            .boundary
            .iter()
            .map(|(owner, port)| owner.parent_port(port).parent.as_ptr())
            .collect();
        for port in ports {
            if !known_parents.contains(&diff.as_ptr()) {
                self.boundary.push((diff.clone(), port));
            }
        }
    }
}

#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;
    use petgraph::dot::{Config, Dot};
    use rstest::rstest;

    use crate::port_diff::tests::TestPortDiff;

    use super::super::tests::root_diff;
    use super::*;

    #[rstest]
    fn test_hierarchy_graph(root_diff: TestPortDiff) {
        let (n0, n1, _, _) = root_diff.nodes().collect_tuple().unwrap();
        let child_a = root_diff.identity_subgraph([n0, n1]);
        let child_aa = child_a.identity_subgraph([n0]);

        let graph = hierarchy_graph([&child_aa]);
        assert_debug_snapshot!(Dot::with_config(
            &graph,
            &[Config::NodeNoLabel, Config::EdgeNoLabel]
        ));
    }

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
        assert_eq!(child_a.exclude_nodes.len(), 2);
        assert_eq!(child_b.exclude_nodes.len(), 3);
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
