//! Serialization of `EdgeData`
//!
//! Default serialization does not work as the bimap type has non-string keys.

use bimap::BiBTreeMap;
use serde::{Deserialize, Serialize};

use crate::{
    port::{BoundaryIndex, Port},
    subgraph::Subgraph,
    Graph,
};

use super::EdgeData;

#[derive(Serialize, Deserialize)]
#[serde(bound(
    serialize = "G::Node: Serialize, G::Edge: Serialize",
    deserialize = "G::Node: Deserialize<'de>, G::Edge: Deserialize<'de>"
))]
struct SerialEdgeData<G: Graph> {
    subgraph: Subgraph<G>,
    port_map: Vec<(Port<G>, BoundaryIndex)>,
}

impl<G: Graph> From<EdgeData<G>> for SerialEdgeData<G> {
    fn from(edge_data: EdgeData<G>) -> Self {
        SerialEdgeData {
            subgraph: edge_data.subgraph,
            port_map: edge_data.port_map.into_iter().collect(),
        }
    }
}

impl<G: Graph> From<SerialEdgeData<G>> for EdgeData<G> {
    fn from(serial: SerialEdgeData<G>) -> Self {
        EdgeData {
            subgraph: serial.subgraph,
            port_map: BiBTreeMap::from_iter(serial.port_map),
        }
    }
}

impl<G: Graph> Serialize for EdgeData<G>
where
    G::Node: Serialize,
    G::Edge: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        SerialEdgeData::from(self.clone()).serialize(serializer)
    }
}

impl<'de, G: Graph> Deserialize<'de> for EdgeData<G>
where
    G::Node: Deserialize<'de>,
    G::Edge: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        SerialEdgeData::deserialize(deserializer).map(Into::into)
    }
}
