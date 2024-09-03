use crux_core::{render::Render, App};
use derive_more::{From, Into};
use portdiff::GraphView;
use portgraph::PortGraph;
use serde::{Deserialize, Serialize};
use tket2::static_circ::StaticSizeCircuit;

use crate::{capability::LogCapability, Model, ViewModel};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    DeserializeData { data: String, format: String },
    SetSelected(Vec<DiffId>),
}

#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, From, Into,
)]
#[serde(transparent)]
pub struct DiffId(pub(crate) u32);

#[cfg_attr(feature = "typegen", derive(crux_core::macros::Export))]
#[derive(crux_core::macros::Effect)]
pub struct Capabilities {
    render: Render<Event>,
    log: LogCapability<Event>,
}

#[derive(Default)]
pub struct PortDiffViewer;

impl App for PortDiffViewer {
    type Event = Event;
    type Model = Model;
    type ViewModel = ViewModel;
    type Capabilities = Capabilities;

    fn update(&self, event: Self::Event, model: &mut Self::Model, caps: &Self::Capabilities) {
        match event {
            Event::DeserializeData { data, format } => match format.as_str() {
                "portgraph" => match serde_json::from_str::<GraphView<PortGraph>>(&data) {
                    Ok(diffs) => model.load(diffs),
                    Err(err) => {
                        caps.log.error(format!("{:?}", err));
                        model.clear()
                    }
                },
                "tket" => match serde_json::from_str::<GraphView<StaticSizeCircuit>>(&data) {
                    Ok(diffs) => model.load(diffs),
                    Err(err) => {
                        caps.log.error(format!("{:?}", err));
                        model.clear()
                    }
                },
                _ => {
                    caps.log.error(format!("Unsupported format: {}", format));
                }
            },
            Event::SetSelected(ids) => model.set_selected(ids.into_iter().collect()),
        };

        let mut n_trimmed = 0;
        while !model.are_compatible() {
            model.trim_selected(1);
            n_trimmed += 1;
        }
        if n_trimmed > 0 {
            caps.log.error(format!(
                "Incompatible diffs. Trimmed {} incompatible diffs",
                n_trimmed
            ));
        }

        caps.render.render();
    }

    fn view(&self, model: &Self::Model) -> Self::ViewModel {
        model.current_view().unwrap_or(ViewModel::Loaded {
            graph: "error".to_string(),
            graph_type: "tket",
            hierarchy: vec![],
            selected: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crux_core::testing::AppTester;
    use rstest::rstest;

    use crate::{model::LoadedModel, view_serialise::RFGraph};

    use super::*;

    #[test]
    fn test_app_empty() {
        let app = AppTester::<PortDiffViewer, _>::default();
        let mut model = Model::None;
        app.update(
            Event::DeserializeData {
                data: "".to_string(),
                format: "portgraph".to_string(),
            },
            &mut model,
        );
        assert!(matches!(model, Model::None));
        let view = app.view(&model);
        assert!(matches!(view, ViewModel::None));
    }

    #[test]
    fn test_app_load_asserts() {
        let app = AppTester::<PortDiffViewer, _>::default();
        let mut model = Model::None;
        app.update(
            Event::DeserializeData {
                data: include_str!("../../../test_files/parent_child.json").to_string(),
                format: "portgraph".to_string(),
            },
            &mut model,
        );
        let Model::Portgraph(LoadedModel {
            selected_diffs,
            all_diffs,
            ..
        }) = &model
        else {
            panic!("expected loaded model");
        };
        assert_eq!(selected_diffs.len(), 1);
        assert_eq!(all_diffs.all_nodes().count(), 2);

        let view = app.view(&model);
        let ViewModel::Loaded {
            graph,
            graph_type,
            hierarchy,
            selected,
        } = view
        else {
            panic!("expected loaded view");
        };
        let graph: RFGraph = serde_json::from_str(&graph).unwrap();
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.edges.len(), 6);
        assert_eq!(graph_type, "portgraph");
        assert_eq!(hierarchy, vec![(DiffId(0), DiffId(1)).into()]);
        assert_eq!(selected, BTreeSet::from([DiffId(1)]));
    }

    #[rstest]
    #[case("parent_child.json")]
    #[case("parent_two_children.json")]
    #[case("parent_two_children_overlapping.json")]
    fn test_app_load_many(#[case] file_name: &str) {
        let app = AppTester::<PortDiffViewer, _>::default();
        let mut model = Model::None;
        let file_path = format!("../../test_files/{}", file_name);
        app.update(
            Event::DeserializeData {
                data: std::fs::read_to_string(&file_path).unwrap(),
                format: "portgraph".to_string(),
            },
            &mut model,
        );
        let Model::Portgraph(LoadedModel { .. }) = &model else {
            panic!("expected loaded model");
        };
        let ViewModel::Loaded { .. } = app.view(&model) else {
            panic!("expected loaded view");
        };
    }

    #[rstest]
    #[case("circ_rewrite.json")]
    #[case("circ_rewrite_2.json")]
    fn test_app_load_circuit(#[case] file_name: &str) {
        let app = AppTester::<PortDiffViewer, _>::default();
        let mut model = Model::None;
        let file_path = format!("../../test_files/{}", file_name);
        let updates = app.update(
            Event::DeserializeData {
                data: std::fs::read_to_string(&file_path).unwrap(),
                format: "tket".to_string(),
            },
            &mut model,
        );
        dbg!(&updates);
        let Model::Tket(..) = &model else {
            panic!("expected loaded model");
        };
        let ViewModel::Loaded { .. } = app.view(&model) else {
            panic!("expected loaded view");
        };
        model.set_selected(BTreeSet::from([DiffId(0)]));
        let ViewModel::Loaded { .. } = app.view(&model) else {
            panic!("expected loaded view");
        };
    }
}
