use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    iter::repeat_with,
    rc::Rc,
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
    current: Rc<PortDiff>,
    committed: HashMap<Uuid, Rc<PortDiff>>,
    vertex_origin: HashMap<Uuid, Uuid>,
    current_boundary: Vec<Uuid>,
}

impl AppState {
    fn new(init_diff: Rc<PortDiff>) -> Self {
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

    pub(crate) fn commit(&mut self, port_diff: Rc<PortDiff>) -> Uuid {
        let id = Uuid::new_v4();
        for v in port_diff.vertices() {
            if !self.vertex_origin.contains_key(&v.id()) {
                self.vertex_origin.insert(v.id(), id);
            }
        }
        self.committed.insert(id, port_diff);
        id
    }

    pub(crate) fn init() -> Self {
        let init_diff = examples::port_diff();
        let mut ret = Self::new(init_diff);
        ret.commit_current();
        ret
    }

    pub(crate) fn current(&self) -> &Rc<PortDiff> {
        &self.current
    }

    pub(crate) fn commit_current(&mut self) -> Uuid {
        self.commit(self.current.clone())
    }

    pub(crate) fn set_current(&mut self, diff: Rc<PortDiff>) {
        self.current_boundary = repeat_with(Uuid::new_v4)
            .take(diff.n_boundary_edges())
            .collect_vec();
        self.current = diff;
    }

    pub(crate) fn to_json(&mut self) -> Result<String, String> {
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
        let (boundary_edges, other_edges): (Vec<_>, Vec<_>) = edges.into_iter().partition(|edge| {
            let Some(left) = nodes.iter().find(|n| n.id() == edge.left.node.id()) else {
                return false;
            };
            let Some(right) = nodes.iter().find(|n| n.id() == edge.right.node.id()) else {
                return false;
            };
            (left.is_external() && right.is_internal())
                || (left.is_internal() && right.is_external())
        });
        let boundary_nodes = self
            .current_boundary
            .iter()
            .map(|&id| Node::new_boundary(id))
            .collect_vec();
        if boundary_edges.len() != boundary_nodes.len() {
            return Err("Mismatch between boundary edges and boundary nodes".to_string());
        }
        let split_boundary_edges = boundary_edges
            .into_iter()
            .enumerate()
            .flat_map(|(i, edge)| {
                [
                    Edge::from_boundary(&edge.left, &boundary_nodes[i]),
                    Edge::from_boundary(&edge.right, &boundary_nodes[i]),
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
