use itertools::Itertools;
use portdiff::UniqueVertex;

use crate::{Port, PortDiff, PortEdge, PortLabel};

pub fn port_diff() -> PortDiff {
    let v = (0..(3 + 2 + 3)).map(|_| UniqueVertex::new()).collect_vec();
    let create_edge = |src, src_port, tgt, tgt_port| PortEdge {
        left: Port {
            node: v[src],
            port: PortLabel::Out(src_port),
        },
        right: Port {
            node: v[tgt],
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

// pub(crate) fn graph() -> Graph {
//     let delta = 200;
//     let internals = (0..2)
//         .map(|i| Node::new_internal(0, delta * i + delta))
//         .collect_vec();
//     let externals = (0..3)
//         .map(|i| Node::new_external(delta * i - delta, 0))
//         .chain((0..3).map(|i| Node::new_external(delta * i - delta, 3 * delta)))
//         .collect_vec();
//     let boundary = (0..3)
//         .map(|i| Node::new_boundary(delta * i - delta, delta / 2))
//         .chain((0..3).map(|i| Node::new_boundary(delta * i - delta, 5 * delta / 2)))
//         .collect_vec();
//     let eb1 = externals[0..3]
//         .iter()
//         .zip(&boundary)
//         .map(|(e, b)| Edge::new(e, b));
//     let bi1 = boundary[0..3].iter().map(|b| Edge::new(b, &internals[0]));
//     let ii = [Edge::new(&internals[0], &internals[1])];
//     let ib2 = boundary[3..].iter().map(|b| Edge::new(&internals[1], b));
//     let be2 = boundary[3..]
//         .iter()
//         .zip(&externals[3..])
//         .map(|(b, e)| Edge::new(b, e));
//     let edges = eb1.chain(bi1).chain(ii).chain(ib2).chain(be2).collect_vec();
//     let nodes = internals
//         .into_iter()
//         .chain(externals)
//         .chain(boundary)
//         .collect_vec();
//     Graph::new(nodes, edges)
// }
