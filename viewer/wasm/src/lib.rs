mod app_state;
mod examples;
mod hierarchy;
mod port_diff_id;
mod portdiff_serial;
mod wasm_api;

use app_state::AppState;
use port_diff_id::{PortDiffId, PortDiffIdCreator};

type Port = portdiff::Port<portdiff::DetVertex, PortLabel>;
type PortEdge = portdiff::PortEdge<portdiff::DetVertex, PortLabel>;
type PortDiff = portdiff::PortDiff<portdiff::DetVertex, PortLabel>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PortLabel {
    In(usize),
    Out(usize),
}

impl PortLabel {
    fn index(&self) -> usize {
        match self {
            PortLabel::In(i) | PortLabel::Out(i) => *i,
        }
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;
    use portdiff::{DetVertexCreator, Port};

    use crate::{
        examples::{self, gen_det_vertices},
        PortDiff, PortEdge, PortLabel,
    };

    #[test]
    fn test_e2e() {
        let mut vertex_creator = DetVertexCreator::default();
        let diff = examples::port_diff(gen_det_vertices(&mut vertex_creator));
        let vs = diff
            .vertices()
            .filter(|v| diff.degree(v) == 4)
            .cloned()
            .collect_vec();
        let mut edges = diff
            .internal_edges()
            .map(|e| diff.internal_edge(&e).clone())
            .collect_vec();
        edges.push(PortEdge {
            left: Port {
                node: vs[0].clone(),
                port: PortLabel::In(1),
            },
            right: Port {
                node: vs[1].clone(),
                port: PortLabel::Out(1),
            },
        });
        // Rewrite once
        let diff = diff
            .rewrite(
                &edges,
                &vec![None; diff.n_boundary_edges()],
                &mut vertex_creator,
            )
            .unwrap();
        let edges = diff
            .internal_edges()
            .map(|e| diff.internal_edge(&e).clone())
            .collect_vec();
        // Rewrite twice
        let diff = diff
            .rewrite(
                &edges,
                &vec![None; diff.n_boundary_edges()],
                &mut vertex_creator,
            )
            .unwrap();
        let vs = diff
            .vertices()
            .filter(|v| diff.degree(v) == 5)
            .cloned()
            .collect_vec();
        // Select
        PortDiff::with_nodes([vs[0].clone(), vs[1].clone()], &diff);
    }
}
