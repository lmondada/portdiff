use itertools::Itertools;
use portdiff::{port_diff::MergeType, DetVertex};
use uuid::Uuid;
use wasm_bindgen::prelude::*;

use crate::{port_diff_id::PortDiffId, portdiff_serial::Edge, AppState, PortDiff};

static mut APP_STATE: Option<AppState> = None;

fn app_state<'a>() -> &'a AppState {
    unsafe {
        // We are single threaded
        if APP_STATE.is_none() {
            APP_STATE = Some(AppState::init());
        }
        APP_STATE.as_ref().unwrap()
    }
}

fn app_state_mut<'a>() -> &'a mut AppState {
    unsafe {
        if APP_STATE.is_none() {
            APP_STATE = Some(AppState::init());
        }
        APP_STATE.as_mut().unwrap()
    }
}

#[wasm_bindgen]
pub fn init_app() -> Result<String, String> {
    app_state().to_json()
}

#[wasm_bindgen]
pub fn rewrite(edges: String) -> Result<String, String> {
    let Ok(edges): Result<Vec<Edge>, _> = serde_json::from_str(&edges) else {
        return Err("Error parsing edges".to_string());
    };
    let (port_edges, boundary) = app_state().get_rewrite(edges);
    let Ok(new_diff) = app_state_mut().rewrite(&port_edges, &boundary) else {
        return Err("Error rewriting".to_string());
    };
    app_state_mut().set_current(new_diff);
    app_state_mut().commit_current();
    app_state().to_json()
}

#[wasm_bindgen]
pub fn select_nodes(node_ids: String) -> Result<String, String> {
    let Ok(node_ids): Result<Vec<String>, _> = serde_json::from_str(&node_ids) else {
        return Err("Error parsing node ids".to_string());
    };
    let node_ids = node_ids.into_iter().map(|id| DetVertex(id));
    let new_diff = PortDiff::with_nodes(node_ids, app_state().current());
    app_state_mut().set_current(new_diff);
    app_state_mut().commit_current();
    app_state().to_json()
}

#[wasm_bindgen]
pub fn hierarchy() -> Result<String, String> {
    let edges = app_state().hierarchy();
    let edges = edges.into_iter().map(|(v1, v2)| (v1.0, v2.0)).collect_vec();
    serde_json::to_string(&edges).map_err(|_| "Error serializing hierarchy".to_string())
}

#[wasm_bindgen]
pub fn select_diffs(diff_ids: String) -> Result<String, String> {
    let Ok(diff_ids): Result<Vec<String>, _> = serde_json::from_str(&diff_ids) else {
        return Err("Error parsing diff ids".to_string());
    };
    let diffs = diff_ids
        .into_iter()
        .filter_map(|id| app_state().committed().get(&PortDiffId(id.clone())));
    let merged_diff = PortDiff::merge_all(diffs)?;
    app_state_mut().set_current(merged_diff);
    app_state().to_json()
}

/// The list of diffs that are compatible with the current selection
#[wasm_bindgen]
pub fn list_compatible(selected: String) -> Result<String, String> {
    let Ok(selected_ids): Result<Vec<String>, _> = serde_json::from_str(&selected) else {
        return Err("Error parsing diff ids".to_string());
    };
    let selected_ids = selected_ids
        .into_iter()
        .map(|id| PortDiffId(id.clone()))
        .collect_vec();
    let selected = selected_ids
        .iter()
        .filter_map(|id| app_state().committed().get(id));
    let merged = PortDiff::merge_all(selected)?;
    let other = app_state()
        .committed()
        .keys()
        .filter(|diff_id| !selected_ids.contains(diff_id));

    let compatible = other
        .filter(|diff_id| {
            let diff = app_state().committed().get(diff_id).unwrap();
            diff.merge_type(&merged) != MergeType::NoMerge
        })
        .map(|p_id| p_id.0.clone())
        .collect_vec();
    serde_json::to_string(&compatible).map_err(|_| "Error serializing compatible diffs".to_string())
}

#[wasm_bindgen]
pub fn current_graph() -> Result<String, String> {
    app_state().to_json()
}

#[wasm_bindgen]
pub fn expand_boundary(boundary_id: String) -> Result<(), String> {
    let boundary_id: String = serde_json::from_str(&format!("\"{}\"", boundary_id))
        .map_err(|e| format!("Error parsing boundary id: {}\n{}", boundary_id, e))?;
    let edge = app_state()
        .find_boundary_edge(&boundary_id)
        .ok_or("Boundary edge not found".to_string())?;
    app_state().current().expand(edge).for_each(|diff| {
        app_state_mut().commit(diff);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2e() {
        init_app().unwrap();
        rewrite(r#"[{"id":"b409981c-94f9-4555-9be8-a3a445804b32","source":"00000000-0000-0000-0000-000000000004","target":"00000000-0000-0000-0000-000000000005","sourceHandle":1,"targetHandle":1},{"id":"b3483b6a-2cd0-4b2e-b92c-1d09957e0d3a","source":"00000000-0000-0000-0000-000000000001","target":"00000000-0000-0000-0000-000000000004","sourceHandle":0,"targetHandle":0},{"id":"4963c59c-f2a2-4ef7-9510-5f4afa4454ac","source":"00000000-0000-0000-0000-000000000002","target":"00000000-0000-0000-0000-000000000004","sourceHandle":0,"targetHandle":1},{"id":"e726b6eb-521f-4d4d-835a-2bf88ccb6442","source":"00000000-0000-0000-0000-000000000003","target":"00000000-0000-0000-0000-000000000004","sourceHandle":0,"targetHandle":2},{"id":"ff2475f4-137e-423d-869f-40ed3445363f","source":"00000000-0000-0000-0000-000000000004","target":"00000000-0000-0000-0000-000000000005","sourceHandle":0,"targetHandle":0},{"id":"544ec31c-5b62-4052-a916-43e420aa6aa2","source":"00000000-0000-0000-0000-000000000005","target":"00000000-0000-0000-0000-000000000006","sourceHandle":0,"targetHandle":0},{"id":"87951abc-d47f-4855-af6f-d8470e89bd6b","source":"00000000-0000-0000-0000-000000000005","target":"00000000-0000-0000-0000-000000000007","sourceHandle":1,"targetHandle":0},{"id":"96b412d0-b073-45e5-9da7-081ef52d51d9","source":"00000000-0000-0000-0000-000000000005","target":"00000000-0000-0000-0000-000000000008","sourceHandle":2,"targetHandle":0}]"#.to_string()).unwrap();
        rewrite(r#"[{"id":"7d018c32-4f27-4058-86a6-d443859244a2","source":"00000000-0000-0000-0000-000000000009","target":"00000000-0000-0000-0000-00000000000a","sourceHandle":1,"targetHandle":1},{"id":"a199aad7-5831-4bce-ae16-abee05762369","source":"00000000-0000-0000-0000-000000000001","target":"00000000-0000-0000-0000-000000000009","sourceHandle":0,"targetHandle":0},{"id":"01a7a455-ed85-4c3e-baaf-5b22a25d2ae3","source":"00000000-0000-0000-0000-000000000002","target":"00000000-0000-0000-0000-000000000009","sourceHandle":0,"targetHandle":1},{"id":"9036042b-f0a7-4d09-8041-741203702c32","source":"00000000-0000-0000-0000-000000000003","target":"00000000-0000-0000-0000-000000000009","sourceHandle":0,"targetHandle":2},{"id":"852c2e7a-8589-498d-9dd2-68ac0d878d02","source":"00000000-0000-0000-0000-000000000009","target":"00000000-0000-0000-0000-00000000000a","sourceHandle":0,"targetHandle":0},{"id":"03494975-2349-4ea6-b46e-57caa3951503","source":"00000000-0000-0000-0000-00000000000a","target":"00000000-0000-0000-0000-000000000006","sourceHandle":0,"targetHandle":0},{"id":"5104c008-b708-4f66-8859-058e2ad388de","source":"00000000-0000-0000-0000-00000000000a","target":"00000000-0000-0000-0000-000000000007","sourceHandle":1,"targetHandle":0},{"id":"cbd43b63-69c6-4f60-a685-43d36a04b426","source":"00000000-0000-0000-0000-00000000000a","target":"00000000-0000-0000-0000-000000000008","sourceHandle":2,"targetHandle":0}]"#.to_string()).unwrap();
        select_nodes(
            r#"["00000000-0000-0000-0000-000000000009","00000000-0000-0000-0000-00000000000a"]"#
                .to_string(),
        )
        .unwrap();
    }

    #[test]
    fn test_2() {
        init_app().unwrap();
        select_nodes(r#"["4"]"#.to_string()).unwrap();
        rewrite(r#"[{"id":"61e3ef49-997a-49da-b7ce-98c2af073875","source":"0e3c833a-6858-41da-8cba-9b52e12b61e9","target":"4","sourceHandle":0,"targetHandle":1},{"id":"39f27383-aec7-4f1b-9c8a-d2251b6a7ab7","source":"8","target":"4","sourceHandle":0,"targetHandle":0},{"id":"4c41ba6b-79c4-43a6-accd-434aec00815b","source":"4","target":"9","sourceHandle":0,"targetHandle":0},{"id":"0719ad27-36e6-419f-99f3-33b198386f30","source":"4","target":"10","sourceHandle":1,"targetHandle":0},{"id":"2d2a35a8-bf7c-401a-a864-5ae3ec88e2c6","source":"4","target":"11","sourceHandle":2,"targetHandle":0}]"#.to_string()).unwrap();
        select_diffs(r#"["PORTDIFF_0"]"#.to_string()).unwrap();
        select_nodes(r#"["3"]"#.to_string()).unwrap();
        rewrite(r#"[{"id":"676d9961-8525-4687-8fb8-178ee0cb77b3","source":"6290ba0d-3313-4081-867b-8543dcc52e36","target":"3","sourceHandle":0,"targetHandle":3},{"id":"dfe76710-8ea7-4064-9997-a8037cba2c08","source":"17","target":"3","sourceHandle":0,"targetHandle":0},{"id":"a301fde9-a676-4ee0-a7e8-39d4f1622154","source":"18","target":"3","sourceHandle":0,"targetHandle":1},{"id":"6d101b18-b1f3-4b5f-ac14-68f96c83bd6d","source":"19","target":"3","sourceHandle":0,"targetHandle":2},{"id":"ca71bc81-936e-4544-bc6f-061fd5d8ce1b","source":"3","target":"20","sourceHandle":0,"targetHandle":0}]"#.to_string()).unwrap();
        select_diffs(r#"["PORTDIFF_2","PORTDIFF_4"]"#.to_string()).unwrap();
        let diff_ids = ["PORTDIFF_2", "PORTDIFF_4"];
        let diffs = diff_ids
            .into_iter()
            .filter_map(|id| app_state().committed().get(&PortDiffId(id.to_string())));
        PortDiff::merge_all(diffs).unwrap();
    }
}
