use std::rc::Rc;

use wasm_bindgen::prelude::*;

use crate::{data_serialization::Edge, AppState, PortEdge};

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
pub fn init_app() -> String {
    app_state_mut().to_json()
}

#[wasm_bindgen]
pub fn rewrite(edges: String) -> String {
    let Ok(edges): Result<Vec<Edge>, _> = serde_json::from_str(&edges) else {
        return "Error parsing edges".to_string();
    };
    let (port_edges, boundary) = app_state().get_rewrite(edges);
    let Ok(new_diff) = app_state().current().rewrite(&port_edges, boundary) else {
        return "Error rewriting".to_string();
    };
    let new_id = app_state_mut().add_port_diff(Rc::new(new_diff));
    app_state_mut().set_selected([new_id]);
    app_state_mut().to_json()
}
