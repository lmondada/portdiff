use std::collections::{BTreeMap, BTreeSet};

use bimap::BiBTreeMap;
use thiserror::Error;

use crate::{
    port::{BoundPort, EdgeEnd, Port, Site},
    port_diff::IncomingEdgeIndex,
    subgraph::Subgraph,
    Graph, PortDiff,
};

use super::{BoundaryPort, EdgeData, Owned, PortDiffData};

#[derive(Error, Debug)]
pub enum InvalidRewriteError {
    #[error("{0}")]
    BoundPortsEdge(String),
    #[error("{0}")]
    InvalidEdge(String),
}

impl<G: Graph> PortDiff<G> {
    /// Create a new diff that rewrites `nodes` and `edges` to `new_graph`.
    ///
    /// The returned diff will be a child of all diffs in `nodes`. Edges are
    /// expressed as pairs of ports. The nodes they belong to must be in `nodes`.
    ///
    /// The function `boundary_map` will be called once for every boundary port
    /// of the new diff. It is passed as argument an owned port, the image of
    /// the boundary port in a parent diff. It must return the site of the
    /// boundary port in the new graph, or a sentinel node.
    pub fn rewrite(
        nodes: impl IntoIterator<Item = Owned<G::Node, G>>,
        edges: impl IntoIterator<Item = (Owned<Port<G>, G>, Owned<Port<G>, G>)>,
        new_graph: G,
        mut boundary_map: impl FnMut(Owned<Port<G>, G>) -> BoundaryPort<G>,
    ) -> Result<Self, InvalidRewriteError> {
        // Collect nodes per portdiff
        let nodes: BTreeMap<_, BTreeSet<_>> =
            nodes.into_iter().fold(BTreeMap::new(), |mut map, n| {
                map.entry(n.owner).or_default().insert(n.data);
                map
            });
        // Split edges into edges within and between portdiffs
        let mut internal_edges: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        let mut used_bound_ports: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        let mut used_unbound_ports: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for (left, right) in edges {
            match (left.data, right.data) {
                (Port::Bound(left_port), Port::Bound(right_port)) => {
                    if left.owner != right.owner {
                        return Err(InvalidRewriteError::BoundPortsEdge(
                            "Edges between bound ports must be on the same portdiff".to_string(),
                        ));
                    }
                    if left_port.edge != right_port.edge {
                        return Err(InvalidRewriteError::BoundPortsEdge(
                            "Edges between bound ports must be on the same edge".to_string(),
                        ));
                    }
                    internal_edges
                        .entry(left.owner)
                        .or_default()
                        .insert(left_port.edge);
                }
                (Port::Boundary(left_port), Port::Boundary(right_port)) => {
                    check_valid_edge(&left, &right)?;
                    used_unbound_ports
                        .entry(left.owner)
                        .or_default()
                        .insert(left_port);
                    used_unbound_ports
                        .entry(right.owner)
                        .or_default()
                        .insert(right_port);
                }
                (Port::Boundary(left_port), Port::Bound(right_port)) => {
                    check_valid_edge(&left, &right)?;
                    if left.owner == right.owner {
                        return Err(InvalidRewriteError::BoundPortsEdge(
                            "A bound port may only connect distinct diffs".to_string(),
                        ));
                    }
                    used_unbound_ports
                        .entry(left.owner)
                        .or_default()
                        .insert(left_port);
                    used_bound_ports
                        .entry(right.owner)
                        .or_default()
                        .insert(right_port);
                }
                (Port::Bound(left_port), Port::Boundary(right_port)) => {
                    check_valid_edge(&right, &left)?;
                    if left.owner == right.owner {
                        return Err(InvalidRewriteError::BoundPortsEdge(
                            "A bound port may only connect distinct diffs".to_string(),
                        ));
                    }
                    used_bound_ports
                        .entry(left.owner)
                        .or_default()
                        .insert(left_port);
                    used_unbound_ports
                        .entry(right.owner)
                        .or_default()
                        .insert(right_port);
                }
            }
        }

        // Create the incoming edges between parents and the new diff
        let mut parents = Vec::new();
        let mut boundary = Vec::new();
        for (i, (diff, nodes)) in nodes.into_iter().enumerate() {
            let incoming_edge = IncomingEdgeIndex(i);
            let mut used_bound_ports = used_bound_ports.remove(&diff).unwrap_or_default();
            let mut used_unbound_ports = used_unbound_ports.remove(&diff).unwrap_or_default();

            // Create subgraph
            let edges = internal_edges.remove(&diff).unwrap_or_default();
            let subgraph = Subgraph::new(&diff.graph, nodes, edges);

            // Map boundaries
            let mut port_map = BiBTreeMap::new();
            for b in subgraph.boundary(&diff.graph) {
                if !used_bound_ports.remove(&b) {
                    let port = Port::Bound(b);
                    let site = boundary_map(Owned {
                        data: port,
                        owner: diff.clone(),
                    });
                    let boundary_ind = boundary.len();
                    boundary.push((site, incoming_edge));
                    port_map.insert(port, boundary_ind.into());
                }
            }
            for b in diff.boundary_iter() {
                let Some(site) = diff.boundary_site(b) else {
                    // Sentinel boundaries cannot be rewritten
                    continue;
                };
                if !subgraph.nodes().contains(&site.node) {
                    continue;
                }
                if !used_unbound_ports.remove(&b) {
                    let port = Port::Boundary(b);
                    let site = boundary_map(Owned {
                        data: port,
                        owner: diff.clone(),
                    });
                    let boundary_ind = boundary.len();
                    boundary.push((site, incoming_edge));
                    port_map.insert(port, boundary_ind.into());
                }
            }
            let edge_data = EdgeData { subgraph, port_map };
            parents.push((diff, edge_data));

            // Check that the edges used only valid boundary ports
            if !used_bound_ports.is_empty() {
                return Err(InvalidRewriteError::InvalidEdge(
                    "Cross-diff edge uses invalid boundary port".to_string(),
                ));
            }
            if !used_unbound_ports.is_empty() {
                return Err(InvalidRewriteError::InvalidEdge(
                    "Cross-diff edge uses invalid boundary port".to_string(),
                ));
            }
        }
        if !internal_edges.is_empty() {
            return Err(InvalidRewriteError::InvalidEdge(
                "Edges with no corresponding nodes".to_string(),
            ));
        }
        let data = PortDiffData {
            graph: new_graph,
            boundary,
        };
        Ok(PortDiff::new(data, parents))
    }

    /// Create a new diff that rewrites `edges` to `new_graph`.
    ///
    /// The `nodes` are given by the set of end vertices of the edges. See
    /// [`Self::rewrite`] for more details.
    pub fn rewrite_edges(
        edges: impl IntoIterator<Item = (Owned<Port<G>, G>, Owned<Port<G>, G>)> + Clone,
        new_graph: G,
        boundary_map: impl FnMut(Owned<Port<G>, G>) -> BoundaryPort<G>,
    ) -> Result<Self, InvalidRewriteError> {
        let nodes: BTreeSet<_> = edges
            .clone()
            .into_iter()
            .flat_map(|(l, r)| {
                [l, r].map(|p| Owned {
                    data: p.site().unwrap().node, // TODO: what to do with sentinels?
                    owner: p.owner,
                })
            })
            .collect();
        Self::rewrite(nodes, edges, new_graph, boundary_map)
    }

    /// Create a new diff that rewrites the subgraph of `self` induced by `nodes`.
    ///
    /// See [`Self::rewrite`] for more details.
    pub fn rewrite_induced(
        &self,
        nodes: &BTreeSet<G::Node>,
        new_graph: G,
        mut boundary_map: impl FnMut(Port<G>) -> BoundaryPort<G>,
    ) -> Result<Self, InvalidRewriteError> {
        let edges = self
            .graph()
            .edges_iter()
            .filter(|&e| {
                let left_node = self.graph().incident_node(e, EdgeEnd::Left);
                let right_node = self.graph().incident_node(e, EdgeEnd::Right);
                nodes.contains(&left_node) && nodes.contains(&right_node)
            })
            .map(|edge| {
                let left_port = Port::Bound(BoundPort {
                    edge,
                    end: EdgeEnd::Left,
                });
                let right_port = Port::Bound(BoundPort {
                    edge,
                    end: EdgeEnd::Right,
                });
                (
                    Owned {
                        data: left_port,
                        owner: self.clone(),
                    },
                    Owned {
                        data: right_port,
                        owner: self.clone(),
                    },
                )
            });
        let nodes = nodes.into_iter().copied().map(|data| Owned {
            data,
            owner: self.clone(),
        });
        Self::rewrite(nodes, edges, new_graph, |p| boundary_map(p.data))
    }
}

fn check_valid_edge<G: Graph>(
    left: &Owned<Port<G>, G>,
    right: &Owned<Port<G>, G>,
) -> Result<(), InvalidRewriteError> {
    match left
        .owner
        .opposite_ports(left.data)
        .iter()
        .find(|p| p == &right)
    {
        Some(_) => Ok(()),
        None => Err(InvalidRewriteError::InvalidEdge(
            "Valid edges must have opposite ports".to_string(),
        )),
    }
}

// /// The sets of nodes and edges to be rewritten
// #[derive(Clone)]
// struct RewriteSubgraph<G: Graph> {
//     nodes: BTreeSet<G::Node>,
//     internal_edges: BTreeSet<G::Edge>,
//     new_boundary_ports: BTreeSet<Site<G::Node, G::PortLabel>>,
// }

// impl<G: Graph> Default for RewriteSubgraph<G> {
//     fn default() -> Self {
//         Self {
//             nodes: BTreeSet::new(),
//             internal_edges: BTreeSet::new(),
//             new_boundary_ports: BTreeSet::new(),
//         }
//     }
// }

// impl<G: Graph> RewriteSubgraph<G> {
//     fn collect(
//         nodes: impl IntoIterator<Item = UniqueNodeId<G>>,
//         edges: impl IntoIterator<Item = EdgeData<G>>,
//     ) -> HashMap<PortDiff<G>, Self> {
//         let mut ret_map = HashMap::<PortDiff<G>, Self>::new();

//         for node in nodes.into_iter() {
//             ret_map
//                 .entry(node.owner)
//                 .or_default()
//                 .nodes
//                 .insert(node.node);
//         }

//         for edge in edges {
//             match edge {
//                 EdgeData::Internal { owner, edge } => {
//                     ret_map
//                         .entry(owner)
//                         .or_default()
//                         .internal_edges
//                         .insert(edge);
//                 }
//                 EdgeData::Boundary { left, right } => {
//                     if let Port::Unbound { owner, port } = left {
//                         ret_map
//                             .entry(owner)
//                             .or_default()
//                             .new_boundary_ports
//                             .insert(port);
//                     }
//                     if let Port::Unbound { owner, port } = right {
//                         ret_map
//                             .entry(owner)
//                             .or_default()
//                             .new_boundary_ports
//                             .insert(port);
//                     }
//                 }
//             }
//         }
//         ret_map
//     }

//     fn filter_boundary<'a>(
//         &'a self,
//         boundary: &'a Boundary<G>,
//     ) -> impl Iterator<Item = (Site<G::Node, G::PortLabel>, ParentPort<G>)> + 'a {
//         let mut boundary_ports = self.new_boundary_ports.clone();
//         boundary
//             .iter()
//             .filter(|(port, _)| self.nodes.contains(&port.node))
//             .filter(move |(port, _)| {
//                 // Only keep in boundary if not present in new boundary edges
//                 !boundary_ports.remove(port)
//             })
//             .map(|(port, parent)| (port.clone(), parent.clone()))
//     }

//     fn new_boundary_from_edges<'a>(
//         &'a self,
//         graph: &'a G,
//     ) -> impl Iterator<Item = (Site<G::Node, G::PortLabel>, BoundPort<G::Edge>)> + 'a {
//         graph
//             .edges_iter()
//             .filter(|e| !self.internal_edges.contains(e))
//             .flat_map(move |edge| {
//                 let left_port = graph.get_port_site(BoundPort {
//                     edge,
//                     port: EdgeEnd::Left,
//                 });
//                 let right_port = graph.get_port_site(BoundPort {
//                     edge,
//                     port: EdgeEnd::Right,
//                 });
//                 let mut boundary_ports = Vec::new();
//                 if self.nodes.contains(&left_port.node) {
//                     boundary_ports.push((
//                         left_port,
//                         BoundPort {
//                             edge,
//                             port: EdgeEnd::Left,
//                         },
//                     ));
//                 }
//                 if self.nodes.contains(&right_port.node) {
//                     boundary_ports.push((
//                         right_port,
//                         BoundPort {
//                             edge,
//                             port: EdgeEnd::Right,
//                         },
//                     ));
//                 }
//                 boundary_ports
//             })
//     }
// }

#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use itertools::Itertools;
    use portgraph::{
        render::DotFormat, LinkMut, LinkView, PortGraph, PortMut, PortOffset, PortView,
    };
    use rstest::rstest;

    use crate::{
        port::Port,
        port_diff::tests::{parent_child_diffs, TestPortDiff},
    };

    use super::*;

    #[ignore = "TODO this is currently not deterministic"]
    #[rstest]
    fn test_rewrite(parent_child_diffs: [TestPortDiff; 2]) {
        let [parent, _] = parent_child_diffs;
        let rewrite = |v| {
            let mut rhs = PortGraph::new();
            let n0 = rhs.add_node(0, 4);
            let n1 = rhs.add_node(1, 0);
            rhs.link_nodes(n0, 3, n1, 0).unwrap();
            parent.rewrite_induced(&BTreeSet::from_iter([v]), rhs, |p| {
                let offset = Owned::new(p, parent.clone()).site().unwrap().port;
                Site {
                    node: n0,
                    port: offset,
                }
                .into()
            })
        };
        let (_, n1, n2, _) = PortView::nodes_iter(&parent.graph).collect_tuple().unwrap();
        let child_a = rewrite(n1).unwrap();
        let child_b = rewrite(n2).unwrap();

        let pg: PortGraph =
            PortDiff::extract_graph([child_a.clone(), child_b.clone()].to_vec()).unwrap();
        assert_eq!(pg.node_count(), 6);
        assert_eq!(pg.link_count(), 3 + 3 + 1 + 2);

        // Now rewrite across child_a and child_b
        let mut rhs = PortGraph::new();
        let n0 = rhs.add_node(0, 2);
        let n1 = rhs.add_node(2, 0);
        rhs.link_nodes(n0, 0, n1, 0).unwrap();
        rhs.link_nodes(n0, 1, n1, 1).unwrap();

        let child_a_out0 = child_a
            .boundary_iter()
            .find(|&bd| child_a.boundary_site(bd).unwrap().port == PortOffset::Outgoing(0))
            .unwrap();
        let child_b_in0 = child_b
            .boundary_iter()
            .find(|&bd| child_b.boundary_site(bd).unwrap().port == PortOffset::Incoming(0))
            .unwrap();
        let cross_edge = (
            Owned::new(Port::Boundary(child_a_out0), child_a.clone()),
            Owned::new(Port::Boundary(child_b_in0), child_b.clone()),
        );

        let nodes = BTreeSet::from_iter([
            Owned::new(n0, child_a.clone()),
            Owned::new(n0, child_b.clone()),
        ]);
        let merged = PortDiff::rewrite(nodes, [cross_edge], rhs, |n| {
            if n.owner == child_a {
                Site {
                    node: n0,
                    port: n.site().unwrap().port,
                }
                .into()
            } else {
                Site {
                    node: n1,
                    port: n.site().unwrap().port,
                }
                .into()
            }
        })
        .unwrap();
        let pg: PortGraph = PortDiff::extract_graph([merged].to_vec()).unwrap();
        assert_snapshot!("extracted_graph_2", pg.dot_string());
    }
}
