use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    iter::repeat_with,
    rc::Rc,
};

use itertools::Itertools;
use portdiff::UniqueVertex;
use uuid::Uuid;

use crate::{
    data_serialization::{Edge, Graph, Node},
    examples, Port, PortDiff, PortEdge, PortLabel,
};

pub(crate) struct AppState {
    selected_port_diffs: HashSet<Uuid>,
    all_port_diffs: HashMap<Uuid, Rc<PortDiff>>,
    vertex_origin: HashMap<Uuid, Uuid>,
    current_boundary: Vec<Uuid>,
}

impl AppState {
    fn new() -> Self {
        let selected_port_diffs = HashSet::new();
        let all_port_diffs = HashMap::new();
        let vertex_origin = HashMap::new();
        let current_boundary = Vec::new();
        Self {
            selected_port_diffs,
            all_port_diffs,
            vertex_origin,
            current_boundary,
        }
    }

    pub(crate) fn add_port_diff(&mut self, port_diff: Rc<PortDiff>) -> Uuid {
        let id = Uuid::new_v4();
        for v in port_diff.vertices() {
            if !self.vertex_origin.contains_key(&v.id()) {
                self.vertex_origin.insert(v.id(), id);
            }
        }
        self.all_port_diffs.insert(id, port_diff);
        id
    }

    pub(crate) fn set_selected(&mut self, ids: impl IntoIterator<Item = Uuid>) {
        self.selected_port_diffs = HashSet::from_iter(ids);
    }

    pub(crate) fn init() -> Self {
        let mut ret = Self::new();
        let init_diff = examples::port_diff();
        let init_id = ret.add_port_diff(init_diff);
        ret.set_selected([init_id]);
        ret
    }

    pub(crate) fn current(&self) -> &PortDiff {
        let id = self.selected_port_diffs.iter().exactly_one().unwrap();
        self.all_port_diffs[id].as_ref()
    }

    pub(crate) fn to_json(&mut self) -> String {
        let (g, current_boundary) = self.convert_to_graph(self.current());
        self.current_boundary = current_boundary;
        g.to_json()
    }

    pub(crate) fn convert_to_graph(&self, port_diff: &PortDiff) -> (Graph, Vec<Uuid>) {
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
                            panic!("Unknown vertex");
                        };
                        return Node::new_internal(id, origin);
                    } else {
                        Node::new_external(id)
                    }
                })
            })
            .unique()
            .collect_vec();
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
        let boundary_nodes = repeat_with(|| Node::new_boundary())
            .take(boundary_edges.len())
            .collect_vec();
        let current_boundary = boundary_nodes.iter().map(|n| n.id()).collect_vec();
        let split_boundary_edges = boundary_edges
            .into_iter()
            .enumerate()
            .flat_map(|(i, edge)| {
                [
                    Edge::new(
                        edge.left.node.id(),
                        edge.left.port.index(),
                        boundary_nodes[i].id(),
                        0,
                    ),
                    Edge::new(
                        boundary_nodes[i].id(),
                        0,
                        edge.right.node.id(),
                        edge.right.port.index(),
                    ),
                ]
            })
            .collect_vec();
        nodes.extend(boundary_nodes);
        let edges = split_boundary_edges
            .into_iter()
            .chain(other_edges.into_iter().map(|e| Edge::from(&e)))
            .collect_vec();
        (Graph { nodes, edges }, current_boundary)
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
