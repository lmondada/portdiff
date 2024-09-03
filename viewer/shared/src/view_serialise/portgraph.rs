//! A wrapper around PortGraph for ReactFlow

use portdiff::Graph;
use portgraph::{self as pg, PortGraph};
use serde::{Deserialize, Serialize};

use super::ViewSerialise;

impl ViewSerialise for PortGraph {
    fn graph_type(&self) -> &'static str {
        "portgraph"
    }

    fn to_json(&self) -> String {
        serde_json::to_string(&RFGraph::from(self)).unwrap()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RFNode {
    pub id: String,
    pub num_in_handles: u32,
    pub num_out_handles: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RFEdge {
    pub source: String,
    pub source_handle: u32,
    pub target: String,
    pub target_handle: u32,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RFGraph {
    pub nodes: Vec<RFNode>,
    pub edges: Vec<RFEdge>,
}

impl RFGraph {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<'r> From<&'r pg::PortGraph> for RFGraph {
    fn from(value: &'r pg::PortGraph) -> Self {
        let nodes = value.nodes_iter().map(|n| RFNode {
            id: format!("{:?}", n),
            num_in_handles: <pg::PortGraph as pg::PortView>::num_inputs(value, n) as u32,
            num_out_handles: <pg::PortGraph as pg::PortView>::num_outputs(value, n) as u32,
        });
        let edges = value.edges_iter().map(|e| RFEdge {
            source: format!("{:?}", e.out_node()),
            source_handle: e.out_offset().index() as u32,
            target: format!("{:?}", e.in_node(value)),
            target_handle: e.in_offset(value).index() as u32,
        });
        RFGraph {
            nodes: nodes.collect(),
            edges: edges.collect(),
        }
    }
}
