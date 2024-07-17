use std::cmp;

use crate::{
    graph::GraphBuilder,
    port::{BoundPort, PortSide, UnboundPort},
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

    fn to_unbound(
        &self,
        BoundPort { edge, port }: BoundPort<Self::Edge>,
    ) -> UnboundPort<Self::Node, Self::PortLabel> {
        let left = self.output(edge.node, edge.outgoing as usize).unwrap();
        let (left, right) = self.port_links(left).exactly_one().unwrap();
        let port_index = match port {
            PortSide::Left => left,
            PortSide::Right => right,
        };
        UnboundPort {
            node: self.port_node(port_index).unwrap(),
            port: self.port_offset(port_index).unwrap(),
        }
    }
}

impl GraphBuilder<PortGraph> for PortGraph {
    type NodeId = pg::NodeIndex;

    fn new() -> Self {
        PortGraph::new()
    }

    fn add_edge(
        &mut self,
        left: UnboundPort<Self::NodeId, <PortGraph as Graph>::PortLabel>,
        right: UnboundPort<Self::NodeId, <PortGraph as Graph>::PortLabel>,
    ) {
        resize_ports(self, &left, &right);
        self.link_offsets(left.node, left.port, right.node, right.port)
            .unwrap();
        // if let Err(err) = edge_add_success {
        //     // check that the edge that exist is identical
        //     let out_port = self.output(left_node, out as usize).unwrap();
        //     let (_, in_port) = self.port_links(out_port).exactly_one().unwrap();
        //     if right_node != self.port_node(in_port).unwrap()
        //         || pg::PortOffset::Incoming(inc) != self.port_offset(in_port).unwrap()
        //     {
        //         panic!("Different edge already exists: {err:?}")
        //     }
        // }
    }

    fn add_node(&mut self, n: <PortGraph as Graph>::Node) -> Self::NodeId {
        PortMut::add_node(self, 0, 0)
    }
}

fn resize_ports(
    graph: &mut PortGraph,
    left: &UnboundPort<pg::NodeIndex, pg::PortOffset>,
    right: &UnboundPort<pg::NodeIndex, pg::PortOffset>,
) {
    let &UnboundPort {
        node,
        port: pg::PortOffset::Outgoing(out),
    } = left
    else {
        panic!("Edge lhs must be outgoing port")
    };
    if graph.outputs(node).count() <= out as usize {
        graph.set_num_ports(node, graph.num_inputs(node), (out + 1) as usize, |_, _| {})
    }

    let &UnboundPort {
        node,
        port: pg::PortOffset::Incoming(inc),
    } = right
    else {
        panic!("Edge rhs must be incoming port")
    };
    if graph.inputs(node).count() <= inc as usize {
        graph.set_num_ports(node, (inc + 1) as usize, graph.num_outputs(node), |_, _| {})
    }
}

impl PortDiff<PortGraph> {
    pub fn nodes(&self) -> impl Iterator<Item = pg::NodeIndex> + '_ {
        PortView::nodes_iter(self.graph())
    }
}
