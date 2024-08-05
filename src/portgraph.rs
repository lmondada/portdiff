use std::collections::BTreeMap;

use crate::{
    port::{BoundPort, EdgeEnd, Site},
    Graph, PortDiff,
};

use itertools::Itertools;
use pg::{LinkMut, LinkView, PortGraph, PortMut, PortView};
use portgraph as pg;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortgraphEdge {
    outgoing: u16,
    node: pg::NodeIndex,
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
        let mut nodes_map = BTreeMap::new();
        for node in nodes {
            let new_node = self.add_node(0, 0);
            nodes_map.insert(*node, new_node);
        }

        // Add every port in `graph` to `self`
        for port in graph.ports_iter() {
            let src = graph.port_node(port).unwrap();
            let src_offset = graph.port_offset(port).unwrap();
            resize_ports(
                self,
                &Site {
                    node: src,
                    port: src_offset,
                },
            );
            // Add all outgoing edges in `port`.
            if graph.port_direction(port).unwrap() == pg::Direction::Outgoing {
                for (_, tgt) in graph.port_links(port) {
                    let tgt_offset = graph.port_offset(tgt).unwrap();
                    let tgt = graph.port_node(tgt).unwrap();
                    resize_ports(
                        self,
                        &Site {
                            node: tgt,
                            port: tgt_offset,
                        },
                    );
                    self.link_offsets(nodes_map[&src], src_offset, nodes_map[&tgt], tgt_offset)
                        .unwrap();
                }
            }
        }
        nodes_map
    }
}

fn resize_ports(graph: &mut PortGraph, site: &Site<pg::NodeIndex, pg::PortOffset>) {
    let node = site.node;
    let offset = site.port.index();
    let dir = site.port.direction();

    if graph.num_ports(node, dir) <= offset as usize {
        let mut in_ports = graph.num_inputs(node);
        let mut out_ports = graph.num_outputs(node);
        match dir {
            pg::Direction::Incoming => in_ports = offset + 1,
            pg::Direction::Outgoing => out_ports = offset + 1,
        }
        graph.set_num_ports(node, in_ports, out_ports, |_, _| {});
    }
}

impl PortDiff<PortGraph> {
    pub fn nodes(&self) -> impl Iterator<Item = pg::NodeIndex> + '_ {
        PortView::nodes_iter(self.graph())
    }
}
