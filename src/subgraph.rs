use std::collections::BTreeSet;
use std::fmt::Debug;

use crate::{
    port::{BoundPort, EdgeEnd},
    Graph,
};

use derive_where::derive_where;
use serde::{Deserialize, Serialize};

#[derive_where(Clone, Default; G: Graph)]
#[derive_where(Debug; G: Graph, G::Node: Debug, G::Edge: Debug)]
#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "G::Node: Serialize, G::Edge: Serialize",
    deserialize = "G::Node: Deserialize<'de>, G::Edge: Deserialize<'de>"
))]
pub struct Subgraph<G: Graph> {
    nodes: BTreeSet<G::Node>,
    edges: BTreeSet<G::Edge>,
}

impl<G: Graph> Subgraph<G> {
    pub fn new(graph: &G, nodes: BTreeSet<G::Node>, edges: BTreeSet<G::Edge>) -> Self {
        assert!(incident_edges(graph, &nodes, &edges));
        Self { nodes, edges }
    }

    pub fn nodes(&self) -> &BTreeSet<G::Node> {
        &self.nodes
    }

    pub fn edges(&self) -> &BTreeSet<G::Edge> {
        &self.edges
    }

    pub fn boundary<'a>(&'a self, graph: &'a G) -> impl Iterator<Item = BoundPort<G::Edge>> + 'a {
        self.nodes.iter().flat_map(move |&n| {
            let ports = graph
                .get_sites(n)
                .flat_map(|site| graph.get_bound_ports(site));
            ports.filter(move |p| !self.edges.contains(&p.edge))
        })
    }
}

fn incident_edges<G: Graph>(
    graph: &G,
    nodes: &BTreeSet<G::Node>,
    edges: &BTreeSet<G::Edge>,
) -> bool {
    for &e in edges {
        let n1 = graph.incident_node(e, EdgeEnd::Left);
        let n2 = graph.incident_node(e, EdgeEnd::Right);
        if !nodes.contains(&n1) || !nodes.contains(&n2) {
            return false;
        }
    }
    true
}

#[cfg(feature = "portgraph")]
#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use portgraph::{LinkMut, NodeIndex, PortGraph, PortMut};
    use rstest::{fixture, rstest};

    use crate::portgraph::PortgraphEdge;

    use super::Subgraph;

    #[fixture]
    fn graph() -> PortGraph {
        let mut graph = PortGraph::new();

        let n0 = graph.add_node(0, 3);
        let n1 = graph.add_node(3, 1);
        let n2 = graph.add_node(1, 3);
        let n3 = graph.add_node(3, 0);

        for i in 0..3 {
            graph.link_nodes(n0, i, n1, i).unwrap();
            graph.link_nodes(n2, i, n3, i).unwrap();
        }
        graph.link_nodes(n1, 0, n2, 0).unwrap();
        graph
    }

    #[rstest]
    fn test_boundary_no_edge(graph: PortGraph) {
        let n1 = NodeIndex::new(1);
        let n2 = NodeIndex::new(2);
        let sub = Subgraph::new(
            &graph,
            [n1, n2].into_iter().collect(),
            [].into_iter().collect(),
        );
        let boundary = sub.boundary(&graph).collect_vec();
        assert_eq!(boundary.len(), 8);
        insta::assert_debug_snapshot!(boundary);
    }

    #[rstest]
    fn test_boundary_edge(graph: PortGraph) {
        let n1 = NodeIndex::new(1);
        let n2 = NodeIndex::new(2);
        let sub = Subgraph::new(
            &graph,
            [n1, n2].into_iter().collect(),
            [PortgraphEdge::new(n1, 0)].into_iter().collect(),
        );
        let boundary = sub.boundary(&graph).collect_vec();
        assert_eq!(boundary.len(), 6);
        insta::assert_debug_snapshot!(boundary);
    }
}
