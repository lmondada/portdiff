use std::collections::{BTreeMap, BTreeSet};

use crate::{port::BoundPort, Site};

use super::port::EdgeEnd;

/// A graph for port diffing.
///
/// It must be possible to iterate through all nodes and edges of the graph.
/// Furthermore, each edge must distinguish a left end and a right end. This
/// does not have to match the directedness of the edge, but it must be fixed.
///
/// Incident edges can furthermore be distinguished using a port label type,
/// attached to the edge ends.
pub trait Graph: Default + Clone {
    type Node: Ord + Copy;
    type Edge: Ord + Copy;
    type PortLabel: Ord + Clone;

    /// Iterate over all nodes in the graph.
    fn nodes_iter(&self) -> impl Iterator<Item = Self::Node> + '_;

    /// Iterate over all edges in the graph.
    fn edges_iter(&self) -> impl Iterator<Item = Self::Edge> + '_;

    /// Find the site of a bound port.
    ///
    /// There is a unique site for every bound port. The reverse is not
    /// true: site may not have an incident edge, or may have multiple.
    fn get_port_site(&self, bound_port: BoundPort<Self::Edge>)
        -> Site<Self::Node, Self::PortLabel>;

    fn get_bound_ports(
        &self,
        site: Site<Self::Node, Self::PortLabel>,
    ) -> impl Iterator<Item = BoundPort<Self::Edge>> + '_;

    fn get_sites(
        &self,
        node: Self::Node,
    ) -> impl Iterator<Item = Site<Self::Node, Self::PortLabel>> + '_;

    /// The node incident to a given edge and port side.
    ///
    /// This can be obtained from the bound -> unbound port map.
    fn incident_node(&self, edge: Self::Edge, end: EdgeEnd) -> Self::Node {
        let bound_port = BoundPort { edge, end };
        self.get_port_site(bound_port).node
    }

    fn link_sites(
        &mut self,
        left: Site<Self::Node, Self::PortLabel>,
        right: Site<Self::Node, Self::PortLabel>,
    );

    /// Add a subgraph of `graph` to `self`.
    ///
    /// Add the subgraph of `graph` that is induced by `nodes`.
    ///
    /// Return a map from `nodes` in `graph` to the new nodes in `self`.
    fn add_subgraph(
        &mut self,
        graph: &Self,
        nodes: &BTreeSet<Self::Node>,
    ) -> BTreeMap<Self::Node, Self::Node>;
}
