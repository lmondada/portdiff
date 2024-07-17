use std::collections::{BTreeMap, BTreeSet};

use crate::{port::BoundPort, UnboundPort};

use super::port::PortSide;

/// A graph for port diffing.
///
/// It must be possible to iterate through all nodes and edges of the graph.
/// Furthermore, each edge must distinguish a left end and a right end. This
/// does not have to match the directedness of the edge, but it must be fixed.
///
/// Incident edges can furthermore be distinguished using a port label type,
/// attached to the edge ends.
pub trait Graph: Sized {
    type Node: Ord + Copy;
    type Edge: Ord + Copy;
    type PortLabel: Ord + Clone;

    /// Iterate over all nodes in the graph.
    fn nodes_iter(&self) -> impl Iterator<Item = Self::Node> + '_;

    /// Iterate over all edges in the graph.
    fn edges_iter(&self) -> impl Iterator<Item = Self::Edge> + '_;

    /// Convert a bound port (given by edge and port side) to an unbound port
    /// (given by node and port label).
    ///
    /// There is a unique unbound port for every bound port. The reverse is not
    /// true: unbound ports may not have an incident edge, or may have multiple.
    fn to_unbound(
        &self,
        bound_port: BoundPort<Self::Edge>,
    ) -> UnboundPort<Self::Node, Self::PortLabel>;

    /// The node incident to a given edge and port side.
    ///
    /// This can be obtained from the bound -> unbound port map.
    fn incident_node(&self, edge: Self::Edge, port: PortSide) -> Self::Node {
        let bound_port = BoundPort { edge, port };
        self.to_unbound(bound_port).node
    }
}

pub trait GraphBuilder<G: Graph> {
    type NodeId: Ord + Copy;

    fn new() -> Self;

    fn add_edge(
        &mut self,
        left: UnboundPort<Self::NodeId, G::PortLabel>,
        right: UnboundPort<Self::NodeId, G::PortLabel>,
    );

    fn add_node(&mut self, node: G::Node) -> Self::NodeId;

    /// Add a subgraph to `self`
    ///
    /// The `nodes` iterator provides all nodes in the subgraph. The `exclude_edges`
    /// iterator provides all edges in the subgraph that should be excluded from
    /// the induced subgraph.
    fn add_subgraph(
        &mut self,
        graph: &G,
        nodes: impl Iterator<Item = G::Node>,
        exclude_edges: impl Iterator<Item = G::Edge>,
    ) -> BTreeMap<G::Node, Self::NodeId>
    where
        Self: Sized,
    {
        let nodes: BTreeSet<_> = nodes.collect();
        let exclude_edges: BTreeSet<_> = exclude_edges.collect();
        let remap: BTreeMap<_, _> = nodes.iter().map(|&n| (n, self.add_node(n))).collect();

        for edge in graph.edges_iter() {
            if exclude_edges.contains(&edge) {
                continue;
            }
            let left = graph.to_unbound(BoundPort {
                edge,
                port: PortSide::Left,
            });
            let right = graph.to_unbound(BoundPort {
                edge,
                port: PortSide::Right,
            });
            if !nodes.contains(&left.node) || !nodes.contains(&right.node) {
                continue;
            }
            let left_node = remap[&left.node];
            let right_node = remap[&right.node];
            self.add_edge(
                UnboundPort {
                    node: left_node,
                    port: left.port,
                },
                UnboundPort {
                    node: right_node,
                    port: right.port,
                },
            );
        }
        remap
    }
}
