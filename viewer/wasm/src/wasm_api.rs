use portdiff::UniqueVertex;
use uuid::Uuid;
use wasm_bindgen::prelude::*;

use crate::{portdiff_serial::Edge, AppState, PortDiff};

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
    let Ok(new_diff) = app_state().current().rewrite(&port_edges, boundary) else {
        return Err("Error rewriting".to_string());
    };
    app_state_mut().set_current(new_diff);
    app_state_mut().commit_current();
    app_state().to_json()
}

#[wasm_bindgen]
pub fn select_nodes(node_ids: String) -> Result<String, String> {
    let Ok(node_ids): Result<Vec<Uuid>, _> = serde_json::from_str(&node_ids) else {
        return Err("Error parsing node ids".to_string());
    };
    let node_ids = node_ids.into_iter().map(|id| UniqueVertex::from_id(id));
    let new_diff = PortDiff::with_nodes(node_ids, app_state().current());
    app_state_mut().set_current(new_diff);
    app_state_mut().commit_current();
    app_state().to_json()
}

#[wasm_bindgen]
pub fn hierarchy() -> Result<String, String> {
    let edges = app_state().hierarchy();
    serde_json::to_string(&edges).map_err(|_| "Error serializing hierarchy".to_string())
}

#[wasm_bindgen]
pub fn select_diffs(diff_ids: String) -> Result<String, String> {
    let Ok(diff_ids): Result<Vec<Uuid>, _> = serde_json::from_str(&diff_ids) else {
        return Err("Error parsing diff ids".to_string());
    };
    let diffs = diff_ids
        .into_iter()
        .filter_map(|id| app_state().committed().get(&id));
    let mut merged_diff: Option<PortDiff> = None;
    for diff in diffs {
        merged_diff = if let Some(merged_diff) = merged_diff {
            merged_diff.merge(&diff)
        } else {
            Some(diff.clone())
        };
        if merged_diff.is_none() {
            return Err("Cannot merge diffs".to_string());
        }
    }
    let Some(merged_diff) = merged_diff else {
        return Err("Cannot merge empty diff set".to_string());
    };
    app_state_mut().set_current(merged_diff);
    app_state().to_json()
}

#[wasm_bindgen]
pub fn current_graph() -> Result<String, String> {
    app_state().to_json()
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
}
