use portdiff as pd;
use portgraph::PortGraph;

use derive_more::From;

#[derive(Clone, From)]
pub enum SupportedGraphViews {
    PortGraph(pd::GraphView<PortGraph>),
}
