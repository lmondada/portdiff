use std::collections::BTreeSet;

use derive_more::From;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use portdiff::{port_diff::IncompatiblePortDiff, GraphView, NodeId, PortDiff};
use portgraph::PortGraph;
use serde::{Deserialize, Serialize};

use crate::{rfgraph::RFGraph, DiffId};

type Diffs = GraphView<PortGraph>;
type DiffPtr = NodeId<PortGraph>;

#[derive(Default, From)]
pub enum Model {
    #[default]
    None,
    Loaded(LoadedModel),
}

pub struct LoadedModel {
    pub(crate) selected_diffs: BTreeSet<DiffId>,
    pub(crate) diff_id_to_ptr: Vec<DiffPtr>,
    pub(crate) all_diffs: Diffs,
}

// TODO: Check if this is actually safe. We're overriding the safety check here.
unsafe impl Send for Model {}
unsafe impl Sync for Model {}

impl LoadedModel {
    fn extract_graph(&self) -> Result<PortGraph, IncompatiblePortDiff> {
        let node_ids = self
            .selected_diffs
            .iter()
            .map(|diff| self.diff_id_to_ptr[diff.0 as usize]);
        let diffs = node_ids.map(|n| self.all_diffs.get_diff(n)).collect();
        PortDiff::extract_graph(diffs)
    }

    fn hierarchy(&self) -> impl Iterator<Item = HierarchyEdge> + '_ {
        self.all_diffs.inner().edge_references().map(|e| {
            let (src, dst) = (e.source(), e.target());
            let find_pos =
                |n: DiffPtr| self.diff_id_to_ptr.iter().position(|&id| id == n).unwrap() as u32;

            (find_pos(src.into()).into(), find_pos(dst.into()).into()).into()
        })
    }
}

impl Model {
    /// Extract the current graph given by the selected diffs
    pub fn current_view(&self) -> Result<ViewModel, IncompatiblePortDiff> {
        match self {
            Model::None => Ok(ViewModel::None),
            Model::Loaded(model) => {
                let graph = RFGraph::from(&model.extract_graph()?);
                let selected = model.selected_diffs.clone();
                let hierarchy = model.hierarchy().collect();
                Ok(ViewModel::Loaded {
                    graph,
                    selected,
                    hierarchy,
                })
            }
        }
    }

    pub fn load(&mut self, new_diffs: GraphView<PortGraph>) {
        let sinks: BTreeSet<DiffPtr> = new_diffs.sinks().map(|d| (&d).into()).collect();
        let mut selected_diffs = BTreeSet::new();
        let mut diff_id_to_ptr = Vec::new();
        for diff in new_diffs.all_nodes() {
            if sinks.contains(&diff) {
                selected_diffs.insert((diff_id_to_ptr.len() as u32).into());
            }
            diff_id_to_ptr.push(diff);
        }
        *self = LoadedModel {
            selected_diffs,
            diff_id_to_ptr,
            all_diffs: new_diffs,
        }
        .into();
    }

    pub fn set_selected(&mut self, ids: BTreeSet<DiffId>) {
        if let Model::Loaded(model) = self {
            model.selected_diffs = ids;
        }
    }

    pub fn clear(&mut self) {
        *self = Model::None;
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum ViewModel {
    #[default]
    None,
    Loaded {
        graph: RFGraph,
        hierarchy: Vec<HierarchyEdge>,
        selected: BTreeSet<DiffId>,
    },
}

#[derive(Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub struct HierarchyEdge {
    pub parent: DiffId,
    pub child: DiffId,
}

impl From<(DiffId, DiffId)> for HierarchyEdge {
    fn from(pair: (DiffId, DiffId)) -> Self {
        Self {
            parent: pair.0,
            child: pair.1,
        }
    }
}
