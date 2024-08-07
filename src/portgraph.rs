use std::collections::BTreeMap;

use crate::{
    port::{BoundPort, EdgeEnd, Site},
    Graph, PortDiff,
};

use itertools::Itertools;
use pg::{LinkMut, LinkView, PortGraph, PortMut, PortView};
use portgraph as pg;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PortgraphEdge {
    outgoing: u16,
    node: pg::NodeIndex,
}

impl PortgraphEdge {
    pub fn new(node: pg::NodeIndex, outgoing: u16) -> Self {
        Self { node, outgoing }
    }

    pub fn out_node(&self) -> pg::NodeIndex {
        self.node
    }

    pub fn out_offset(&self) -> pg::PortOffset {
        pg::PortOffset::Outgoing(self.outgoing)
    }

    pub fn in_node(&self, g: &PortGraph) -> pg::NodeIndex {
        let port = g.port_index(self.out_node(), self.out_offset()).unwrap();
        let in_port = g.port_link(port).unwrap();
        g.port_node(in_port).unwrap()
    }

    pub fn in_offset(&self, g: &PortGraph) -> pg::PortOffset {
        let port = g.port_index(self.out_node(), self.out_offset()).unwrap();
        let in_port = g.port_link(port).unwrap();
        g.port_offset(in_port).unwrap()
    }
}

impl TryFrom<(pg::NodeIndex, pg::PortOffset)> for PortgraphEdge {
    type Error = &'static str;

    fn try_from((node, port): (pg::NodeIndex, pg::PortOffset)) -> Result<Self, Self::Error> {
        let outgoing = match port {
            pg::PortOffset::Outgoing(out) => out,
            _ => return Err("Invalid edge"),
        };
        Ok(Self { outgoing, node })
    }
}

impl Graph for pg::PortGraph {
    type Node = pg::NodeIndex;

    type Edge = PortgraphEdge; // Using outgoing port offset as edge indices

    type PortLabel = pg::PortOffset;

    fn nodes_iter(&self) -> impl Iterator<Item = Self::Node> + '_ {
        PortView::nodes_iter(self)
    }

    fn edges_iter(&self) -> impl Iterator<Item = Self::Edge> + '_ {
        PortView::nodes_iter(self)
            .flat_map(|node| self.links(node, pg::Direction::Outgoing))
            .map(|(outp, _)| {
                PortgraphEdge::try_from((
                    self.port_node(outp).unwrap(),
                    self.port_offset(outp).unwrap(),
                ))
                .unwrap()
            })
    }

    fn get_port_site(
        &self,
        BoundPort { edge, end }: BoundPort<Self::Edge>,
    ) -> Site<Self::Node, Self::PortLabel> {
        let left = self.output(edge.node, edge.outgoing as usize).unwrap();
        let (left, right) = self.port_links(left).exactly_one().unwrap();
        let port_index = match end {
            EdgeEnd::Left => left,
            EdgeEnd::Right => right,
        };
        Site {
            node: self.port_node(port_index).unwrap(),
            port: self.port_offset(port_index).unwrap(),
        }
    }

    fn get_bound_ports(
        &self,
        unbound_port: Site<Self::Node, Self::PortLabel>,
    ) -> impl Iterator<Item = BoundPort<Self::Edge>> + '_ {
        let port_index = self.port_index(unbound_port.node, unbound_port.port);
        port_index.into_iter().flat_map(|port| {
            self.port_links(port).map(|(src, tgt)| {
                match self.port_offset(src).unwrap().direction() {
                    pg::Direction::Incoming => {
                        let edge = PortgraphEdge::try_from((
                            self.port_node(tgt).unwrap(),
                            self.port_offset(tgt).unwrap(),
                        ))
                        .unwrap();
                        let end = EdgeEnd::Right;
                        BoundPort { edge, end }
                    }
                    pg::Direction::Outgoing => {
                        let edge = PortgraphEdge::try_from((
                            self.port_node(src).unwrap(),
                            self.port_offset(src).unwrap(),
                        ))
                        .unwrap();
                        let end = EdgeEnd::Left;
                        BoundPort { edge, end }
                    }
                }
            })
        })
    }

    fn get_sites(
        &self,
        node: Self::Node,
    ) -> impl Iterator<Item = Site<Self::Node, Self::PortLabel>> + '_ {
        self.all_port_offsets(node)
            .map(move |port| Site { node, port })
    }

    fn link_sites(
        &mut self,
        left: Site<Self::Node, Self::PortLabel>,
        right: Site<Self::Node, Self::PortLabel>,
    ) -> (BoundPort<Self::Edge>, BoundPort<Self::Edge>) {
        let (outport, _) = self
            .link_offsets(left.node, left.port, right.node, right.port)
            .unwrap();
        let edge = (
            self.port_node(outport).unwrap(),
            self.port_offset(outport).unwrap(),
        )
            .try_into()
            .unwrap();
        (
            BoundPort {
                edge,
                end: EdgeEnd::Left,
            },
            BoundPort {
                edge,
                end: EdgeEnd::Right,
            },
        )
    }

    fn add_subgraph(
        &mut self,
        graph: &Self,
        nodes: &std::collections::BTreeSet<Self::Node>,
    ) -> std::collections::BTreeMap<Self::Node, Self::Node> {
        println!("Adding subgraph with {} nodes", nodes.len());
        let mut nodes_map = BTreeMap::new();
        println!("N nodes in self: {}", self.node_count());
        for node in nodes {
            let new_node = self.add_node(0, 0);
            nodes_map.insert(*node, new_node);
        }
        println!(
            "N nodes in self after adding subgraph: {}",
            self.node_count()
        );

        // Add every port in `graph` to `self`
        for (&node, &self_node) in &nodes_map {
            self.set_num_ports(
                self_node,
                graph.num_inputs(node),
                graph.num_outputs(node),
                |_, _| {},
            );

            // Add all outgoing edges of `node` with target in `nodes`.
            for port in graph.all_ports(node) {
                let offset = graph.port_offset(port).unwrap();
                for (_, other_port) in graph.port_links(port) {
                    let other_node = graph.port_node(other_port).unwrap();
                    let Some(other_self_node) = nodes_map.get(&other_node) else {
                        // Ignore edges not in induced subgraph
                        continue;
                    };
                    let other_offset = graph.port_offset(other_port).unwrap();
                    if (other_node, other_offset) <= (node, offset) {
                        // By only adding the edge when other is smaller, we
                        // i) avoid duplicating edges and ii) we know that the
                        // other ports have already been resized.
                        self.link_offsets(self_node, offset, *other_self_node, other_offset)
                            .unwrap();
                    }
                }
            }
        }
        nodes_map
    }
}

impl PortDiff<PortGraph> {
    pub fn nodes(&self) -> impl Iterator<Item = pg::NodeIndex> + '_ {
        PortView::nodes_iter(self.graph())
    }
}
