mod app_state;
mod examples;
mod portdiff_serial;
mod wasm_api;

use app_state::AppState;

type Port = portdiff::Port<portdiff::UniqueVertex, PortLabel>;
type PortEdge = portdiff::PortEdge<portdiff::UniqueVertex, PortLabel>;
type PortDiff = portdiff::PortDiff<portdiff::UniqueVertex, PortLabel>;

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
    use portdiff::Port;

    use crate::{examples, PortDiff, PortEdge, PortLabel};

    #[test]
    fn test_e2e() {
        let diff = examples::port_diff();
        let vs = diff
            .vertices()
            .filter(|v| diff.degree(v) == 4)
            .copied()
            .collect_vec();
        let mut edges = diff
            .internal_edges()
            .map(|e| diff.internal_edge(&e).clone())
            .collect_vec();
        edges.push(PortEdge {
            left: Port {
                node: vs[0],
                port: PortLabel::In(1),
            },
            right: Port {
                node: vs[1],
                port: PortLabel::Out(1),
            },
        });
        // Rewrite once
        let diff = diff
            .rewrite(&edges, vec![None; diff.n_boundary_edges()])
            .unwrap();
        let edges = diff
            .internal_edges()
            .map(|e| diff.internal_edge(&e).clone())
            .collect_vec();
        // Rewrite twice
        let diff = diff
            .rewrite(&edges, vec![None; diff.n_boundary_edges()])
            .unwrap();
        let vs = diff
            .vertices()
            .filter(|v| diff.degree(v) == 5)
            .copied()
            .collect_vec();
        // Select
        PortDiff::with_nodes([vs[0], vs[1]], &diff);
    }
}
