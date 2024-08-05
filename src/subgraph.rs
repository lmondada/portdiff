use std::collections::BTreeSet;

use crate::{
    port::{BoundPort, EdgeEnd},
    Graph,
};

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

impl<G: Graph> Clone for Subgraph<G> {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            edges: self.edges.clone(),
        }
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
