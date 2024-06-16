use std::iter::repeat_with;

use itertools::Itertools;
use portdiff::{DetVertex, DetVertexCreator, Port, PortDiff, PortEdge};

use crate::PortLabel;

pub fn gen_det_vertices(creator: &mut DetVertexCreator) -> impl Iterator<Item = DetVertex> + '_ {
    repeat_with(|| creator.create())
}

pub fn port_diff<V: Clone + Ord>(gen_vertices: impl Iterator<Item = V>) -> PortDiff<V, PortLabel> {
    let v = gen_vertices.take(3 + 2 + 3).collect_vec();
    let create_edge = |src: usize, src_port, tgt: usize, tgt_port| PortEdge {
        left: Port {
            node: v[src].clone(),
            port: PortLabel::Out(src_port),
        },
        right: Port {
            node: v[tgt].clone(),
            port: PortLabel::In(tgt_port),
        },
    };
    let edges = (0..3)
        .map(|i| create_edge(i, 0, 3, i))
        .chain([create_edge(3, 0, 4, 0)])
        .chain((5..8).map(|i| create_edge(4, i - 5, i, 0)))
        .collect_vec();
    PortDiff::with_no_boundary(edges)
}
