use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    iter::repeat_with,
};

use itertools::Itertools;
use portdiff::{BoundaryEdge, DetVertex, DetVertexCreator};

use crate::{
    examples::{self, gen_det_vertices},
    portdiff_serial::{Edge, Graph, Node},
    Port, PortDiff, PortDiffId, PortDiffIdCreator, PortEdge, PortLabel,
};

type NodeId = String;

pub(crate) struct AppState {
    current: PortDiff,
    committed: HashMap<PortDiffId, PortDiff>,
    /// Map from vertex ID to the port diff that created them
    vertex_origin: HashMap<NodeId, PortDiffId>,
    /// The ordered IDs of the current boundary
    current_boundary: Vec<NodeId>,
    /// Vertex creator
    vertex_creator: DetVertexCreator,
    /// PortDiffId creator
    port_diff_id_creator: PortDiffIdCreator,
}

impl AppState {
    fn new(init_diff: PortDiff, vertex_creator: DetVertexCreator) -> Self {
        let all_port_diffs = HashMap::new();
        let vertex_origin = HashMap::new();
        let current_boundary = Vec::new();
        let port_diff_id_creator = PortDiffIdCreator::default();
        Self {
            current: init_diff.clone(),
            committed: all_port_diffs,
            vertex_origin,
            current_boundary,
            vertex_creator,
            port_diff_id_creator,
        }
    }

    pub(crate) fn commit(&mut self, port_diff: PortDiff) -> PortDiffId {
        let id = self.port_diff_id_creator.create();
        for v in port_diff.vertices() {
            if !self.vertex_origin.contains_key(v.id()) {
                self.vertex_origin.insert(v.id().to_string(), id.clone());
            }
        }
        self.committed.insert(id.clone(), port_diff);
        id
    }

    pub(crate) fn committed(&self) -> &HashMap<PortDiffId, PortDiff> {
        &self.committed
    }

    pub(crate) fn vertex_origin(&self) -> &HashMap<NodeId, PortDiffId> {
        &self.vertex_origin
    }

    pub(crate) fn find_boundary_edge(&self, boundary_id: &str) -> Option<BoundaryEdge> {
        self.current_boundary
            .iter()
            .position(|id| id == boundary_id)
            .map(BoundaryEdge::from)
    }

    pub(crate) fn init() -> Self {
        let mut vertex_creator = DetVertexCreator::new();
        let init_diff = examples::port_diff(gen_det_vertices(&mut vertex_creator));
        let mut ret = Self::new(init_diff, vertex_creator);
        ret.commit_current();
        ret
    }

    pub(crate) fn current(&self) -> &PortDiff {
        &self.current
    }

    pub(crate) fn commit_current(&mut self) -> PortDiffId {
        self.commit(self.current.clone())
    }

    pub(crate) fn set_current(&mut self, diff: PortDiff) {
        self.current_boundary = repeat_with(|| self.vertex_creator.create().0)
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
        let internal_edges: BTreeSet<_> = port_diff
            .internal_edges()
            .map(|e| port_diff.internal_edge(&e))
            .collect();
        // Construct nodes, distinguish internal & external
        let mut nodes = edges
            .iter()
            .flat_map(|edge| {
                let left_id = edge.left.node.id();
                let right_id = edge.right.node.id();
                [left_id, right_id].into_iter().map(|id| {
                    if internal_vertices.contains(&id) {
                        let Some(origin) = self.vertex_origin.get(id) else {
                            return Err(format!("Unknown vertex: {}", id));
                        };
                        return Ok(Node::new_internal(id.to_string(), origin.clone()));
                    } else {
                        return Ok(Node::new_external(id.to_string()));
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
            .map(|id| Node::new_boundary(id.to_string()))
            .collect_vec();
        let mut boundary_edges_map: BTreeMap<usize, Vec<&Node>> = BTreeMap::new();
        for (&ind, boundary) in boundary_edges_ind.iter().zip(boundary_nodes.iter()) {
            boundary_edges_map.entry(ind).or_default().push(boundary);
        }
        let other_edges = edges
            .iter()
            .enumerate()
            .filter(|(i, _)| !boundary_edges_map.contains_key(i))
            .map(|(_, e)| e.clone())
            .collect_vec();
        let split_boundary_edges = boundary_edges_map
            .into_iter()
            .flat_map(|(index, boundary_nodes)| {
                assert!(!boundary_nodes.is_empty());
                let mut new_edges = Vec::new();
                let left_internal = nodes
                    .iter()
                    .filter(|n| matches!(n, Node::Internal(_)))
                    .find(|n| n.id() == edges[index].left.node.id())
                    .is_some();
                new_edges.push(Edge::from_boundary(
                    &edges[index].left,
                    boundary_nodes.first().unwrap(),
                    if left_internal {
                        None
                    } else {
                        Some("dashed".to_string())
                    },
                ));
                for (bd1, bd2) in boundary_nodes.iter().tuple_windows() {
                    if matches!(&edges[index].left.port, PortLabel::Out(_)) {
                        new_edges.push(Edge::from_nodes(bd1, bd2, Some("dashed".to_string())));
                    } else {
                        new_edges.push(Edge::from_nodes(bd2, bd1, Some("dashed".to_string())));
                    }
                }
                new_edges.push(Edge::from_boundary(
                    &edges[index].right,
                    boundary_nodes.last().unwrap(),
                    if boundary_nodes.len() < 2 && left_internal {
                        Some("dashed".to_string())
                    } else {
                        None
                    },
                ));
                new_edges
            })
            .collect_vec();
        nodes.extend(boundary_nodes);
        let edges = split_boundary_edges
            .into_iter()
            .chain(other_edges.into_iter().map(|e| {
                let mut ret = Edge::from(&e);
                if !internal_edges.contains(&e) {
                    ret.style = Some("dashed".to_string());
                }
                ret
            }))
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
                .map(|(i, id)| (id, i)),
        );
        for edge in &edges {
            if let Some(&j) = curr_boundaries.get(&edge.source) {
                boundary[j] = Some(Port {
                    node: DetVertex(edge.target.clone()),
                    port: PortLabel::In(edge.target_handle),
                });
            } else if let Some(&j) = curr_boundaries.get(&edge.target) {
                boundary[j] = Some(Port {
                    node: DetVertex(edge.source.clone()),
                    port: PortLabel::Out(edge.source_handle),
                });
            } else {
                port_edges.push(edge.into());
            }
        }
        (port_edges, boundary)
    }

    pub(crate) fn rewrite(
        &mut self,
        edges: &[PortEdge],
        boundary: &[Option<Port>],
    ) -> Result<PortDiff, String> {
        self.current
            .rewrite(&edges, boundary, &mut self.vertex_creator)
    }
}

/// TODO: handle several identical ports
fn get_boundary_edges(edges: &[PortEdge], port_diff: &PortDiff) -> Result<Vec<usize>, String> {
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
            .next()
            .ok_or(format!(
                "no matching port, port: {:?}\nn_boundaries: {}",
                port,
                port_diff.n_boundary_edges()
            ))?;
        // .exactly_one()
        // .map_err(|_| "ambiguous boundary".to_string())?;
        ret.push(index);
    }
    Ok(ret)
}
