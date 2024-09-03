use portdiff as pd;
use portgraph::PortGraph;

use derive_more::From;
use tket2::static_circ::StaticSizeCircuit;

#[derive(Clone, From)]
pub enum SupportedGraphViews {
    PortGraph(pd::GraphView<PortGraph>),
    Tket(pd::GraphView<StaticSizeCircuit>),
}
