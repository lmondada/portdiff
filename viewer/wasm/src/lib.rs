mod data_serialization;
mod examples;

use wasm_bindgen::prelude::*;

type Port = portdiff::Port<portdiff::UniqueVertex, PortLabel>;
type PortEdge = portdiff::PortEdge<portdiff::UniqueVertex, PortLabel>;
type PortDiff = portdiff::PortDiff<portdiff::UniqueVertex, PortLabel>;

/// Returns a simple greeting with the provided name.
///
/// `name` - The name to use in the greeting.
#[wasm_bindgen]
pub fn graph() -> String {
    let g = examples::graph();
    g.to_json()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum PortLabel {
    In(usize),
    Out(usize),
}
