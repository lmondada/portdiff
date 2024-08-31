//! Serialisation of supported graph types for display in react

mod portgraph;
mod supported_formats;

pub use supported_formats::SupportedGraphViews;

pub use portgraph::{RFEdge, RFGraph, RFNode};

pub trait ViewSerialise {
    /// The type of graph this object serialises to
    fn graph_type(&self) -> &'static str;

    /// The json serialisation of the graph
    fn to_json(&self) -> String;
}
