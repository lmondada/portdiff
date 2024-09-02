use std::{collections::BTreeSet, iter};

use derive_more::From;
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use portdiff::{self as pd, port_diff::IncompatiblePortDiff, GraphView, NodeId, PortDiff};
use portgraph::PortGraph;
use serde::{Deserialize, Serialize};
use tket2::static_circ::StaticSizeCircuit;

use crate::{
    view_serialise::{SupportedGraphViews, ViewSerialise},
    DiffId,
};

type Diffs<G> = GraphView<G>;
type DiffPtr<G> = NodeId<G>;

#[derive(Default, From)]
pub enum Model {
    #[default]
    None,
    Portgraph(LoadedModel<PortGraph>),
    Tket(LoadedModel<StaticSizeCircuit>),
}

impl std::fmt::Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Model::None => write!(f, "None"),
            Model::Portgraph(..) => write!(f, "Loaded Portgraph model"),
            Model::Tket(..) => write!(f, "Loaded Tket model"),
        }
    }
}

pub struct LoadedModel<G: pd::Graph> {
    pub(crate) selected_diffs: BTreeSet<DiffId>,
    pub(crate) diff_id_to_ptr: Vec<DiffPtr<G>>,
    pub(crate) all_diffs: Diffs<G>,
}

// TODO: Check if this is actually safe. We're overriding the safety check here.
unsafe impl Send for Model {}
unsafe impl Sync for Model {}

impl<G: pd::Graph> LoadedModel<G> {
    fn extract_graph(&self) -> Result<G, IncompatiblePortDiff> {
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
                |n: DiffPtr<G>| self.diff_id_to_ptr.iter().position(|&id| id == n).unwrap() as u32;

            (find_pos(src.into()).into(), find_pos(dst.into()).into()).into()
        })
    }

    fn current_view(&self) -> Result<ViewModel, IncompatiblePortDiff>
    where
        G: ViewSerialise,
    {
        let graph = self.extract_graph()?;
        let graph_type = graph.graph_type();
        let selected = self.selected_diffs.clone();
        let hierarchy = self.hierarchy().collect();
        Ok(ViewModel::Loaded {
            graph: graph.to_json(),
            graph_type,
            selected,
            hierarchy,
        })
    }

    fn load(all_diffs: GraphView<G>) -> Self {
        let sinks: BTreeSet<DiffPtr<G>> = all_diffs.sinks().map(|d| (&d).into()).collect();
        let mut selected_diffs = BTreeSet::new();
        let mut diff_id_to_ptr = Vec::new();
        for diff in all_diffs.all_nodes() {
            if sinks.contains(&diff) {
                selected_diffs.insert((diff_id_to_ptr.len() as u32).into());
            }
            diff_id_to_ptr.push(diff);
        }
        LoadedModel {
            selected_diffs,
            diff_id_to_ptr,
            all_diffs,
        }
    }

    fn are_compatible(&self) -> bool {
        let node_ids = self
            .selected_diffs
            .iter()
            .map(|diff| self.diff_id_to_ptr[diff.0 as usize]);
        let diffs: Vec<_> = node_ids.map(|n| self.all_diffs.get_diff(n)).collect();
        PortDiff::are_compatible(&diffs)
    }

    fn trim_selected(&mut self, n: usize) {
        for _ in 0..n {
            self.selected_diffs.pop_first();
        }
    }
}

fn as_opt<V, E>(r: Result<Option<V>, E>) -> Option<Result<V, E>> {
    match r {
        Ok(None) => None,
        Ok(Some(v)) => Some(Ok(v)),
        Err(e) => Some(Err(e)),
    }
}

fn is_acyclic(diffs: Vec<PortDiff<StaticSizeCircuit>>) -> bool {
    let circ = PortDiff::extract_graph(diffs).unwrap();
    let mut cmds = circ.commands();
    iter::from_fn(|| as_opt(cmds.try_next())).all(|x| x.is_ok())
}

impl Model {
    /// Extract the current graph given by the selected diffs
    pub fn current_view(&self) -> Result<ViewModel, IncompatiblePortDiff> {
        match self {
            Model::None => Ok(ViewModel::None),
            Model::Portgraph(model) => model.current_view(),
            Model::Tket(model) => model.current_view(),
        }
    }

    pub fn load(&mut self, new_diffs: impl Into<SupportedGraphViews>) {
        *self = match new_diffs.into() {
            SupportedGraphViews::PortGraph(g) => LoadedModel::load(g).into(),
            SupportedGraphViews::Tket(circ) => LoadedModel::load(circ).into(),
        };
    }

    pub fn set_selected(&mut self, ids: BTreeSet<DiffId>) {
        match self {
            Model::Portgraph(model) => model.selected_diffs = ids,
            Model::Tket(model) => model.selected_diffs = ids,
            Model::None => return,
        }
    }

    pub fn clear(&mut self) {
        *self = Model::None;
    }

    pub(crate) fn are_compatible(&self) -> bool {
        match self {
            Model::None => true,
            Model::Portgraph(model) => model.are_compatible(),
            Model::Tket(model) => {
                model.are_compatible() && {
                    let node_ids = model
                        .selected_diffs
                        .iter()
                        .map(|diff| model.diff_id_to_ptr[diff.0 as usize]);
                    let diffs: Vec<_> = node_ids.map(|n| model.all_diffs.get_diff(n)).collect();
                    is_acyclic(diffs)
                }
            }
        }
    }

    /// Remove the first `n` elements from the selected diffs
    pub(crate) fn trim_selected(&mut self, n: usize) {
        match self {
            Model::None => return,
            Model::Portgraph(model) => model.trim_selected(n),
            Model::Tket(model) => model.trim_selected(n),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub enum ViewModel {
    #[default]
    None,
    Loaded {
        graph: String,
        graph_type: &'static str,
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
