mod app_state;
mod data_serialization;
mod examples;
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
