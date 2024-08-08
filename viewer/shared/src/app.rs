use crux_core::{render::Render, App};
use derive_more::{From, Into};
use serde::{Deserialize, Serialize};

use crate::{capability::LogCapability, Model, ViewModel};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    DeserializeData(String),
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
            Event::DeserializeData(data) => match serde_json::from_str(&data) {
                Ok(diffs) => model.load(diffs),
                Err(err) => {
                    caps.log.error(format!("{:?}", err));
                    model.clear()
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
        model.current_view().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use crux_core::testing::AppTester;
    use rstest::rstest;

    use crate::model::LoadedModel;

    use super::*;

    #[test]
    fn test_app_empty() {
        let app = AppTester::<PortDiffViewer, _>::default();
        let mut model = Model::None;
        app.update(Event::DeserializeData("".to_string()), &mut model);
        assert!(matches!(model, Model::None));
        let view = app.view(&model);
        assert!(matches!(view, ViewModel::None));
    }

    #[test]
    fn test_app_load_asserts() {
        let app = AppTester::<PortDiffViewer, _>::default();
        let mut model = Model::None;
        app.update(
            Event::DeserializeData(
                include_str!("../../../test_files/parent_child.json").to_string(),
            ),
            &mut model,
        );
        let Model::Loaded(LoadedModel {
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
            hierarchy,
            selected,
        } = view
        else {
            panic!("expected loaded view");
        };
        assert_eq!(graph.nodes.len(), 4);
        assert_eq!(graph.edges.len(), 6);
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
            Event::DeserializeData(std::fs::read_to_string(&file_path).unwrap()),
            &mut model,
        );
        let Model::Loaded(LoadedModel { .. }) = &model else {
            panic!("expected loaded model");
        };
        let ViewModel::Loaded { .. } = app.view(&model) else {
            panic!("expected loaded view");
        };
    }
}
