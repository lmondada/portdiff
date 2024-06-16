use std::collections::HashMap;

use crate::{port_diff_id::PortDiffId, AppState, PortDiff};

impl AppState {
    /// Compute the hierarchy of committed diffs.
    pub(crate) fn hierarchy(&self) -> Vec<(PortDiffId, PortDiffId)> {
        let vertices = self.committed().keys().cloned().collect::<Vec<_>>();
        let vertex_origin: HashMap<_, _> = self
            .vertex_origin()
            .iter()
            .map(|(v, diff_id)| {
                let diff_ind = vertices.iter().position(|uuid| uuid == diff_id).unwrap();
                (v.clone(), diff_ind)
            })
            .collect();

        let diffs = vertices
            .iter()
            .map(|v| self.committed().get(v).unwrap())
            .collect::<Vec<_>>();
        let adj = adjacency_matrix(&diffs, &vertex_origin);
        let trans_adj = transitive_closure(&adj);
        let min_adj = transitive_reduction(&adj, &trans_adj);

        let mut edges = vec![];
        for i in 0..diffs.len() {
            for j in 0..diffs.len() {
                if min_adj[i][j] {
                    edges.push((vertices[i].clone(), vertices[j].clone()));
                }
            }
        }
        edges
    }
}

fn adjacency_matrix(
    diffs: &[&PortDiff],
    vertex_origins: &HashMap<String, usize>,
) -> Vec<Vec<bool>> {
    let n = diffs.len();
    let mut adj = vec![vec![false; n]; n];
    for i in 0..n {
        adj[i][i] = true;
    }
    add_deps_from_vertices(&mut adj, diffs, vertex_origins);
    add_deps_from_ancestors(&mut adj, diffs);
    adj
}

fn add_deps_from_vertices(
    adj: &mut [Vec<bool>],
    diffs: &[&PortDiff],
    vertex_origins: &HashMap<String, usize>,
) {
    for (i, diff) in diffs.iter().enumerate() {
        for v in diff.vertices() {
            if let Some(&origin) = vertex_origins.get(v.id()) {
                adj[i][origin] = true;
            }
        }
    }
}

fn add_deps_from_ancestors(adj: &mut [Vec<bool>], diffs: &[&PortDiff]) {
    for (i, diff) in diffs.iter().enumerate() {
        for anc in diff.boundary_edges().map(|b| diff.get_ancestor(&b)) {
            if let Some(anc_ind) = diffs.iter().position(|&d| d == anc) {
                adj[i][anc_ind] = true;
            }
        }
    }
}

fn transitive_closure(adj: &Vec<Vec<bool>>) -> Vec<Vec<bool>> {
    let n = adj.len();
    let mut trans_adj = adj.clone();
    loop {
        let mut has_changed = false;
        for i in 0..n {
            for j in 0..n {
                if !trans_adj[i][j] && adj[i][j] {
                    trans_adj[i][j] = true;
                    has_changed = true;
                }
            }
        }
        if !has_changed {
            break;
        }
    }
    trans_adj
}

fn transitive_reduction(adj: &Vec<Vec<bool>>, trans_adj: &Vec<Vec<bool>>) -> Vec<Vec<bool>> {
    let n = adj.len();
    let mut min_adj = adj.clone();
    for i in 0..n {
        for j in 0..n {
            for pred_j in (0..n).filter(|&k| adj[k][j] && k != j && k != i) {
                if trans_adj[i][pred_j] {
                    // We can get from i to j through pred_j
                    min_adj[i][j] = false;
                    break;
                }
            }
        }
    }
    min_adj
}
