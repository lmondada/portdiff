use std::collections::{BTreeMap, BTreeSet};

use petgraph::visit::{EdgeRef, IntoEdges};

use crate::{
    port::{BoundPort, BoundaryIndex, EdgeEnd, Port, Site},
    Graph, GraphView, NodeId, PortDiff,
};

use super::{EdgeData, IncomingEdgeIndex, Owned, PortDiffData};

impl<G: Graph> PortDiff<G> {
    /// Squash all diffs in `graph` into a single equivalent diff.
    ///
    /// The incoming edges of the new diff is the union of the incoming edges into
    /// `graph`. The new diff has no outgoing edges.
    ///
    /// Note: this will panic if the diffs in `graph` are not compatible (the
    /// public-facing [Self::extract_graph] will check for compatibility first).
    pub(crate) fn squash(graph: &GraphView<G>) -> Self {
        let mut builder = Builder::new();

        // For each diff in `graph`, add the subgraph of the replacement graph
        // minus the nodes removed by other diffs in `graph`.
        builder.add_subgraphs(graph);

        builder.flatten_incoming_edges(graph);

        // For each boundary port of a node of `graph`, consider whether the port
        // can be resolved within `graph` (i.e. there is a non-boundary ancestor
        // port within `graph`):
        //  - if so, then store the mapping to the resolved port (we will add an
        //    edge in the next step)
        //  - otherwise, add to new boundary.
        let mut resolved_ports_map = BTreeMap::new();

        let all_nodes = graph.all_nodes().collect::<BTreeSet<_>>();
        for &diff_id in &all_nodes {
            let diff = graph.get_diff(diff_id);
            for bd_index in diff.boundary_iter() {
                if !builder.contains(Owned::new(diff.boundary_site(bd_index).node, diff.clone())) {
                    continue;
                }
                match try_resolve_port(Owned::new(bd_index, diff.clone()), &all_nodes) {
                    Ok(bound_port) => {
                        let old_site = diff.boundary_site(bd_index);
                        let new_site = builder
                            .map_site(Owned::new(old_site.clone(), diff.clone()))
                            .unwrap();
                        resolved_ports_map.insert(bound_port, new_site);
                    }
                    Err(boundary) => {
                        let old_site = diff.boundary_site(bd_index);
                        let new_site = builder
                            .map_site(Owned::new(old_site.clone(), diff.clone()))
                            .unwrap();
                        builder.append_boundary(new_site, boundary);
                    }
                }
            }
        }
        // For each resolved port, insert new edges
        while let Some((parent_port, new_site)) = resolved_ports_map.pop_first() {
            let parent_opp_port = parent_port.opposite();
            let new_opp_site =
                if let Some(new_opp_site) = resolved_ports_map.remove(&parent_opp_port) {
                    // The new edge is between two new sites
                    new_opp_site
                } else {
                    // Find (old) opposite site by following the edge in parent and
                    // then translating to the new site with `node_map`
                    builder.map_site(parent_opp_port.site()).expect(
                    "a parent port was neither a boundary port nor a non-rewritten port in child",
                )
                };
            match parent_port.data.end {
                EdgeEnd::Left => {
                    builder.graph.link_sites(new_site, new_opp_site);
                }
                EdgeEnd::Right => {
                    builder.graph.link_sites(new_opp_site, new_site);
                }
            }
        }

        builder.finish()
    }
}

/// Find an ancestor port that is not a boundary port within `all_nodes`.
///
/// If a bound port could not be found, return the last boundary port that
/// is still in `all_nodes`, i.e. it's parent is not in `all_nodes`.
fn try_resolve_port<G: Graph>(
    mut boundary: Owned<BoundaryIndex, G>,
    all_nodes: &BTreeSet<NodeId<G>>,
) -> Result<Owned<BoundPort<G::Edge>, G>, Owned<BoundaryIndex, G>> {
    let mut port = boundary.owner.parent_port(boundary.data);
    while all_nodes.contains(&(&port.owner).into()) {
        match port.data {
            Port::Bound(data) => {
                return Ok(Owned {
                    data,
                    owner: port.owner,
                });
            }
            Port::Boundary(data) => {
                boundary = Owned {
                    data,
                    owner: port.owner.clone(),
                };
                port = port.owner.parent_port(data);
            }
        }
    }
    Err(boundary)
}

struct Builder<G: Graph> {
    /// The new boundary
    boundary: Vec<(Site<G::Node, G::PortLabel>, IncomingEdgeIndex)>,
    /// The new incoming edges and their parent
    incoming_edges: Vec<(PortDiff<G>, EdgeData<G>)>,
    /// For each parent, a map from the old edge index to the new edge index
    edge_index_map: BTreeMap<NodeId<G>, BTreeMap<IncomingEdgeIndex, IncomingEdgeIndex>>,
    /// For each parent, a map from the old node to the new node
    nodes_map: BTreeMap<NodeId<G>, BTreeMap<G::Node, G::Node>>,
    /// The new replacement graph
    graph: G,
}

impl<G: Graph> Builder<G> {
    fn new() -> Self {
        Self {
            boundary: vec![],
            incoming_edges: vec![],
            edge_index_map: BTreeMap::new(),
            nodes_map: BTreeMap::new(),
            graph: G::default(),
        }
    }

    /// Add the subgraphs of the replacement graphs that are not rewritten within `graph`.
    ///
    /// For each node in `graph`, store a map from nodes in the old graph to nodes
    /// in the new graph.
    fn add_subgraphs(&mut self, graph: &GraphView<G>) {
        for diff_id in graph.all_nodes() {
            let diff = graph.get_diff(diff_id);
            let mut nodes = diff.graph.nodes_iter().collect::<BTreeSet<_>>();
            for edge in graph.inner().edges(diff_id.into()) {
                for n in edge.weight().subgraph.nodes() {
                    if !nodes.remove(&n) {
                        panic!("found incompatible diffs in GraphView");
                    }
                }
            }
            let nodes_map = self.graph.add_subgraph(&diff.graph, &nodes);
            self.nodes_map.insert(diff_id, nodes_map);
        }
    }

    /// Collect all incoming edges into `graph` and flatten into a single list of edges.
    ///
    /// Store a map from the old edge indices to the new edge indices.
    fn flatten_incoming_edges(&mut self, graph: &GraphView<G>) {
        let all_nodes = graph.all_nodes().collect::<BTreeSet<_>>();
        for &diff_id in &all_nodes {
            let mut edge_index_map = BTreeMap::new();
            let diff = graph.get_diff(diff_id);
            for (index, edge) in diff.all_incoming().iter().enumerate() {
                let edge_source: PortDiff<G> = edge.source().clone().into();
                if all_nodes.contains(&(&edge_source).into()) {
                    // internal edge
                    continue;
                }
                let new_index = self.incoming_edges.len();
                self.incoming_edges.push((
                    edge_source,
                    EdgeData {
                        subgraph: edge.value().subgraph.clone(),
                        port_map: Default::default(),
                    },
                ));
                edge_index_map.insert(IncomingEdgeIndex(index), IncomingEdgeIndex(new_index));
            }
            self.edge_index_map.insert(diff_id, edge_index_map);
        }
    }

    /// Add a new boundary site at `site`, identical to the boundary port given by
    /// `port`.
    fn append_boundary(
        &mut self,
        site: Site<G::Node, G::PortLabel>,
        port: Owned<BoundaryIndex, G>,
    ) {
        let Owned { data: port, owner } = port;
        let edge_index = owner.incoming_edge_index(port).unwrap();
        let new_edge_index = self.edge_index_map[&(&owner).into()][&edge_index];

        // Add to boundary
        self.boundary.push((site, new_edge_index));

        // Link the new boundary port to the parent port
        let new_index = self.boundary.len() - 1;
        let parent_port = owner.parent_port(port).data;
        let (_, edge_data) = &mut self.incoming_edges[new_edge_index.0];
        edge_data.port_map.insert(parent_port, new_index.into());
    }

    fn map_site(
        &self,
        site: Owned<Site<G::Node, G::PortLabel>, G>,
    ) -> Option<Site<G::Node, G::PortLabel>> {
        let Owned { data: site, owner } = site;
        site.filter_map_node(|n| self.nodes_map.get(&(&owner).into())?.get(&n).copied())
    }

    fn finish(self) -> PortDiff<G> {
        PortDiff::new(
            PortDiffData {
                graph: self.graph,
                boundary: self.boundary,
            },
            self.incoming_edges,
        )
    }

    /// Whether `node` is also in the new replacement graph.
    fn contains(&self, node: Owned<G::Node, G>) -> bool {
        let Some(diff_nodes) = self.nodes_map.get(&(&node.owner).into()) else {
            return false;
        };
        diff_nodes.contains_key(&node.data)
    }
}
