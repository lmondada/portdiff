use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
};

use itertools::Itertools;

use crate::{
    port::{BoundPort, ParentPort, PortSide, UnboundPort},
    Graph, PortDiff,
};

use super::{Boundary, PortDiffData, PortDiffEdge, UniqueNodeId};

impl<G: Graph> PortDiff<G> {
    pub fn merge_rewrite(
        nodes: BTreeSet<UniqueNodeId<G>>,
        edges: impl IntoIterator<Item = PortDiffEdge<G>>,
        new_graph: G,
        boundary_map: impl FnMut(UniqueNodeId<G>) -> G::Node,
    ) -> Result<Self, String> {
        // Construct maps from port diff pointers to their corresponding nodes and
        // edges.
        let rewrite_nodes_edges = RewriteSubgraph::collect(nodes.iter().cloned(), edges);
        if !PortDiff::are_compatible(rewrite_nodes_edges.keys()) {
            return Err("Cannot rewrite incompatible port diffs".to_string());
        }

        Ok(create_rewrite(
            rewrite_nodes_edges,
            new_graph,
            nodes,
            boundary_map,
        ))
    }

    pub fn merge_rewrite_unchecked(
        nodes: BTreeSet<UniqueNodeId<G>>,
        edges: impl IntoIterator<Item = PortDiffEdge<G>>,
        new_graph: G,
        boundary_map: impl FnMut(UniqueNodeId<G>) -> G::Node,
    ) -> Self {
        let rewrite_nodes_edges = RewriteSubgraph::collect(nodes.iter().cloned(), edges);
        create_rewrite(rewrite_nodes_edges, new_graph, nodes, boundary_map)
    }

    /// Create a new diff that is a rewrite of `self`.
    ///
    /// The returned diff will be a `child` of `self` and will replace all
    /// `nodes` and `edges` in `self` with `new_graph`.
    ///
    /// A map from the boundary nodes in `self` to the boundary nodes in
    /// `new_graph` must be provided.
    pub fn rewrite(
        &self,
        nodes: impl IntoIterator<Item = G::Node>,
        edges: impl IntoIterator<Item = G::Edge>,
        new_graph: G,
        mut boundary_map: impl FnMut(G::Node) -> G::Node,
    ) -> Result<Self, String> {
        let nodes = nodes
            .into_iter()
            .map(|n| UniqueNodeId::new(n, self.clone()))
            .collect();
        let edges = edges.into_iter().map(|e| PortDiffEdge::Internal {
            owner: self.clone(),
            edge: e,
        });
        Self::merge_rewrite(nodes, edges, new_graph, |n: UniqueNodeId<G>| {
            boundary_map(n.node)
        })
    }

    /// Create a new diff that is a rewrite of `self` on an induced graph.
    ///
    /// The returned diff will replace the graph induced by `nodes` in `self`
    /// with `new_graph`.
    ///
    /// A map from the boundary nodes in `self` to the boundary nodes in
    /// `new_graph` must be provided.
    pub fn rewrite_induced(
        &self,
        nodes: &BTreeSet<G::Node>,
        new_graph: G,
        boundary_map: impl FnMut(G::Node) -> G::Node,
    ) -> Result<Self, String> {
        let edges = self.graph().edges_iter().filter(|&e| {
            let left_node = self.graph().incident_node(e, PortSide::Left);
            let right_node = self.graph().incident_node(e, PortSide::Right);
            nodes.contains(&left_node) && nodes.contains(&right_node)
        });
        self.rewrite(nodes.iter().copied(), edges, new_graph, boundary_map)
    }

    fn create_parent_port(&self, edge: G::Edge, port_side: PortSide) -> ParentPort<G> {
        ParentPort {
            parent: self.clone(),
            port: BoundPort {
                edge,
                port: port_side,
            },
        }
    }

    fn unique_node_ids(&self) -> impl Iterator<Item = UniqueNodeId<G>> + '_ {
        self.graph
            .nodes_iter()
            .map(|n| UniqueNodeId::new(n, self.clone()))
    }

    fn remap_port(
        &self,
        port: &UnboundPort<G::Node, G::PortLabel>,
        mut boundary_map: impl FnMut(UniqueNodeId<G>) -> G::Node,
    ) -> UnboundPort<G::Node, G::PortLabel> {
        let node_id = UniqueNodeId::new(port.node, self.clone());
        let new_node = boundary_map(node_id);
        UnboundPort {
            node: new_node,
            port: port.port.clone(),
        }
    }
}

fn create_rewrite<G: Graph>(
    subgraphs: HashMap<PortDiff<G>, RewriteSubgraph<G>>,
    rhs: G,
    exclude_nodes: BTreeSet<UniqueNodeId<G>>,
    mut boundary_map: impl FnMut(UniqueNodeId<G>) -> G::Node,
) -> PortDiff<G> {
    // Compute the new boundary of the child graph
    // It is composed of:
    //  (i)  boundary ports of parent graphs that are within `nodes`, minus
    //       ports of the new boundary edges.
    let parent_boundaries = subgraphs.iter().flat_map(|(diff, subgraph)| {
        subgraph
            .filter_boundary(&diff.boundary)
            .map(|(port, parent)| (diff.remap_port(&port, &mut boundary_map), parent))
            .collect_vec()
    });
    let mut boundary: BTreeMap<_, _> = parent_boundaries.collect();

    //  (ii) internal edges incident on `nodes` that are not in `edges`.
    let new_boundary_ports = subgraphs.iter().flat_map(|(diff, subgraph)| {
        subgraph
            .new_boundary_from_edges(&diff.graph)
            .map(|(unbound_port, bound_port)| {
                (
                    diff.remap_port(&unbound_port, &mut boundary_map),
                    bound_port.to_parent_port(diff.clone()),
                )
            })
            .collect_vec()
    });
    boundary.extend(new_boundary_ports);

    PortDiff::new(PortDiffData {
        graph: rhs,
        boundary,
        children: RefCell::new(BTreeMap::new()),
        exclude_nodes,
    })
}

/// The sets of nodes and edges to be rewritten
#[derive(Clone)]
struct RewriteSubgraph<G: Graph> {
    nodes: BTreeSet<G::Node>,
    internal_edges: BTreeSet<G::Edge>,
    new_boundary_ports: BTreeSet<UnboundPort<G::Node, G::PortLabel>>,
}

impl<G: Graph> Default for RewriteSubgraph<G> {
    fn default() -> Self {
        Self {
            nodes: BTreeSet::new(),
            internal_edges: BTreeSet::new(),
            new_boundary_ports: BTreeSet::new(),
        }
    }
}

impl<G: Graph> RewriteSubgraph<G> {
    fn collect(
        nodes: impl IntoIterator<Item = UniqueNodeId<G>>,
        edges: impl IntoIterator<Item = PortDiffEdge<G>>,
    ) -> HashMap<PortDiff<G>, Self> {
        let mut ret_map = HashMap::<PortDiff<G>, Self>::new();

        for node in nodes.into_iter() {
            ret_map
                .entry(node.owner)
                .or_default()
                .nodes
                .insert(node.node);
        }

        for edge in edges {
            match edge {
                PortDiffEdge::Internal { owner, edge } => {
                    ret_map
                        .entry(owner)
                        .or_default()
                        .internal_edges
                        .insert(edge);
                }
                PortDiffEdge::Boundary {
                    left_owner,
                    left_port,
                    right_owner,
                    right_port,
                } => {
                    ret_map
                        .entry(left_owner)
                        .or_default()
                        .new_boundary_ports
                        .insert(left_port);
                    ret_map
                        .entry(right_owner)
                        .or_default()
                        .new_boundary_ports
                        .insert(right_port);
                }
            }
        }
        ret_map
    }

    fn filter_boundary<'a>(
        &'a self,
        boundary: &'a Boundary<G>,
    ) -> impl Iterator<Item = (UnboundPort<G::Node, G::PortLabel>, ParentPort<G>)> + 'a {
        let mut boundary_ports = self.new_boundary_ports.clone();
        boundary
            .iter()
            .filter(|(port, _)| self.nodes.contains(&port.node))
            .filter(move |(port, _)| {
                // Only keep in boundary if not present in new boundary edges
                !boundary_ports.remove(port)
            })
            .map(|(port, parent)| (port.clone(), parent.clone()))
    }

    fn new_boundary_from_edges<'a>(
        &'a self,
        graph: &'a G,
    ) -> impl Iterator<Item = (UnboundPort<G::Node, G::PortLabel>, BoundPort<G::Edge>)> + 'a {
        graph
            .edges_iter()
            .filter(|e| !self.internal_edges.contains(e))
            .flat_map(move |edge| {
                let left_port = graph.to_unbound(BoundPort {
                    edge,
                    port: PortSide::Left,
                });
                let right_port = graph.to_unbound(BoundPort {
                    edge,
                    port: PortSide::Right,
                });
                let mut boundary_ports = Vec::new();
                if self.nodes.contains(&left_port.node) {
                    boundary_ports.push((
                        left_port,
                        BoundPort {
                            edge,
                            port: PortSide::Left,
                        },
                    ));
                }
                if self.nodes.contains(&right_port.node) {
                    boundary_ports.push((
                        right_port,
                        BoundPort {
                            edge,
                            port: PortSide::Right,
                        },
                    ));
                }
                boundary_ports
            })
    }
}

#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use insta::{assert_debug_snapshot, assert_snapshot};
    use portgraph::{
        render::DotFormat, LinkMut, LinkView, PortGraph, PortMut, PortOffset, PortView,
    };
    use rstest::rstest;

    use crate::port_diff::tests::{root_diff, TestPortDiff};

    use super::*;

    #[rstest]
    fn test_rewrite(root_diff: TestPortDiff) {
        let rewrite = |v| {
            let mut rhs = PortGraph::new();
            let n0 = rhs.add_node(0, 4);
            let n1 = rhs.add_node(4, 0);
            rhs.link_nodes(n0, 3, n1, 3).unwrap();
            root_diff.rewrite_induced(&BTreeSet::from_iter([v]), rhs, |_| n0)
        };
        let (_, n1, n2, _) = PortView::nodes_iter(&root_diff.graph)
            .collect_tuple()
            .unwrap();
        let child_a = rewrite(n1).unwrap();
        let child_b = rewrite(n2).unwrap();

        let pg: PortGraph = PortDiff::extract_graph(&[child_a.clone(), child_b.clone()]).unwrap();
        assert_eq!(pg.node_count(), 6);
        assert_eq!(pg.link_count(), 3 + 3 + 1 + 2);
        assert_snapshot!("extracted_graph_1", pg.dot_string());

        // Now rewrite across child_a and child_b
        let mut rhs = PortGraph::new();
        let n0 = rhs.add_node(0, 1);
        let n1 = rhs.add_node(1, 0);
        rhs.link_nodes(n0, 0, n1, 0).unwrap();

        let cross_edge = PortDiffEdge::Boundary {
            left_owner: child_a.clone(),
            left_port: UnboundPort {
                node: n0,
                port: PortOffset::Outgoing(0),
            },
            right_owner: child_b.clone(),
            right_port: UnboundPort {
                node: n0,
                port: PortOffset::Incoming(0),
            },
        };

        let nodes = BTreeSet::from_iter([
            UniqueNodeId::new(n0, child_a.clone()),
            UniqueNodeId::new(n0, child_b.clone()),
        ]);
        let merged = PortDiff::merge_rewrite(nodes, [cross_edge], rhs, |n| {
            if n.owner == child_a {
                n0
            } else {
                n1
            }
        })
        .unwrap();
        let pg: PortGraph = PortDiff::extract_graph(&[merged]).unwrap();
        assert_snapshot!("extracted_graph_2", pg.dot_string());
    }
}
