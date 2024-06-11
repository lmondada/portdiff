use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    iter::repeat_with,
};

use itertools::Itertools;
use portdiff::UniqueVertex;
use uuid::Uuid;

use crate::{
    examples,
    portdiff_serial::{Edge, Graph, Node},
    Port, PortDiff, PortEdge, PortLabel,
};

pub(crate) struct AppState {
    current: PortDiff,
    committed: HashMap<Uuid, PortDiff>,
    vertex_origin: HashMap<Uuid, Uuid>,
    current_boundary: Vec<Uuid>,
}

impl AppState {
    fn new(init_diff: PortDiff) -> Self {
        let all_port_diffs = HashMap::new();
        let vertex_origin = HashMap::new();
        let current_boundary = Vec::new();
        Self {
            current: init_diff.clone(),
            committed: all_port_diffs,
            vertex_origin,
            current_boundary,
        }
    }

    pub(crate) fn commit(&mut self, port_diff: PortDiff) -> Uuid {
        let id = Uuid::new_v4();
        for v in port_diff.vertices() {
            if !self.vertex_origin.contains_key(&v.id()) {
                self.vertex_origin.insert(v.id(), id);
            }
        }
        self.committed.insert(id, port_diff);
        id
    }

    pub(crate) fn committed(&self) -> &HashMap<Uuid, PortDiff> {
        &self.committed
    }

    pub(crate) fn vertex_origin(&self) -> &HashMap<Uuid, Uuid> {
        &self.vertex_origin
    }

    pub(crate) fn init() -> Self {
        let init_diff = examples::port_diff();
        let mut ret = Self::new(init_diff);
        ret.commit_current();
        ret
    }

    pub(crate) fn current(&self) -> &PortDiff {
        &self.current
    }

    pub(crate) fn commit_current(&mut self) -> Uuid {
        self.commit(self.current.clone())
    }

    pub(crate) fn set_current(&mut self, diff: PortDiff) {
        self.current_boundary = repeat_with(Uuid::new_v4)
            .take(diff.n_boundary_edges())
            .collect_vec();
        self.current = diff;
    }

    pub(crate) fn to_json(&self) -> Result<String, String> {
        let g = self.convert_to_graph(self.current())?;
        Ok(g.to_json())
    }

    pub(crate) fn convert_to_graph(&self, port_diff: &PortDiff) -> Result<Graph, String> {
        let edges = port_diff.extract();
        let internal_vertices = BTreeSet::from_iter(port_diff.vertices().map(|v| v.id()));
        // Construct nodes, distinguish internal & external
        let mut nodes = edges
            .iter()
            .flat_map(|edge| {
                let left_id = edge.left.node.id();
                let right_id = edge.right.node.id();
                [left_id, right_id].into_iter().map(|id| {
                    if internal_vertices.contains(&id) {
                        let Some(&origin) = self.vertex_origin.get(&id) else {
                            return Err(format!("Unknown vertex: {}", id));
                        };
                        return Ok(Node::new_internal(id, origin));
                    } else {
                        return Ok(Node::new_external(id));
                    }
                })
            })
            .unique()
            .collect::<Result<Vec<_>, _>>()?;
        // Add a boundary for every edge between an internal and external node
        let boundary_edges_ind = get_boundary_edges(&edges, port_diff)?;
        let boundary_nodes = self
            .current_boundary
            .iter()
            .map(|&id| Node::new_boundary(id))
            .collect_vec();
        let other_edges = edges
            .iter()
            .enumerate()
            .filter(|(i, _)| !boundary_edges_ind.contains(i))
            .map(|(_, e)| e.clone())
            .collect_vec();
        let split_boundary_edges = boundary_edges_ind
            .into_iter()
            .zip(&boundary_nodes)
            .flat_map(|(index, boundary)| {
                [
                    Edge::from_boundary(&edges[index].left, boundary),
                    Edge::from_boundary(&edges[index].right, boundary),
                ]
            })
            .collect_vec();
        nodes.extend(boundary_nodes);
        let edges = split_boundary_edges
            .into_iter()
            .chain(other_edges.into_iter().map(|e| Edge::from(&e)))
            .collect_vec();
        Ok(Graph { nodes, edges })
    }

    pub(crate) fn get_rewrite(&self, edges: Vec<Edge>) -> (Vec<PortEdge>, Vec<Option<Port>>) {
        let mut port_edges = Vec::new();
        let mut boundary = vec![None; self.current_boundary.len()];
        let curr_boundaries = BTreeMap::from_iter(
            self.current_boundary
                .iter()
                .enumerate()
                .map(|(i, &id)| (id, i)),
        );
        for edge in &edges {
            if let Some(&j) = curr_boundaries.get(&edge.source) {
                boundary[j] = Some(Port {
                    node: UniqueVertex::from_id(edge.target),
                    port: PortLabel::In(edge.target_handle),
                });
            } else if let Some(&j) = curr_boundaries.get(&edge.target) {
                boundary[j] = Some(Port {
                    node: UniqueVertex::from_id(edge.source),
                    port: PortLabel::Out(edge.source_handle),
                });
            } else {
                port_edges.push(edge.into());
            }
        }
        (port_edges, boundary)
    }
}

/// TODO: handle several identical ports
fn get_boundary_edges(
    edges: &[PortEdge],
    port_diff: &portdiff::PortDiff<UniqueVertex, PortLabel>,
) -> Result<Vec<usize>, String> {
    let internal_edges: BTreeSet<_> = port_diff
        .internal_edges()
        .map(|e| port_diff.internal_edge(&e))
        .collect();
    let mut ret = Vec::with_capacity(port_diff.n_boundary_edges());
    for b_edge in port_diff.boundary_edges() {
        let port = port_diff.boundary_edge(&b_edge);
        let (index, _) = edges
            .iter()
            .enumerate()
            .filter(|(_, e)| &e.left == port || &e.right == port)
            .filter(|(_, e)| !internal_edges.contains(e))
            .exactly_one()
            .map_err(|_| "ambiguous boundary".to_string())?;
        ret.push(index);
    }
    Ok(ret)
}
