use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;
use petgraph::visit::{EdgeRef, IntoEdges};
use union_find::{QuickUnionUf, UnionBySize, UnionFind};

use crate::{
    port::{BoundPort, BoundaryIndex, EdgeEnd, Port, Site},
    Graph, NodeId, PortDiff, PortDiffGraph,
};

use super::{BoundarySite, EdgeData, IncomingEdgeIndex, Owned, PortDiffData};

impl<G: Graph> PortDiff<G> {
    /// Squash all diffs in `graph` into a single equivalent diff.
    ///
    /// The incoming edges of the new diff is the union of the incoming edges into
    /// `graph`. The new diff has no outgoing edges.
    ///
    /// Note: this will panic if the diffs in `graph` are not compatible (the
    /// public-facing [Self::extract_graph] will check for compatibility first).
    pub(crate) fn squash(graph: &PortDiffGraph<G>) -> Self {
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
        let mut new_wire_id = 0; // Give each wire a unique id
        for &diff_id in &all_nodes {
            let diff = graph.get_diff(diff_id);
            let mut wire_map = BTreeMap::new(); // Map wire ids in diff to new wires
            for bd_index in diff.boundary_iter() {
                let old_site = diff.boundary_site(bd_index);
                let new_site = match old_site.clone().try_into_site() {
                    Ok(site) => {
                        let Some(site) = builder.map_site(Owned::new(site, diff.clone())) else {
                            // Site is outside of the rewritten region.
                            continue;
                        };
                        site.into()
                    }
                    Err(BoundarySite::Wire { id, end }) => {
                        // Map wire ID (diff local) to a new wire ID (graph-wide unique).
                        let id = *wire_map.entry(id).or_insert_with(|| {
                            let id = new_wire_id;
                            new_wire_id += 1;
                            id
                        });
                        BoundarySite::Wire { id, end }
                    }
                    Err(_) => unreachable!(),
                };

                match try_resolve_port(Owned::new(bd_index, diff.clone()), &all_nodes) {
                    Ok(bound_port) => {
                        resolved_ports_map.insert(bound_port, new_site);
                    }
                    Err(boundary) => {
                        builder.append_boundary(new_site, boundary);
                    }
                }
            }
        }

        builder.add_boundary_edges(resolved_ports_map);

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
    boundary: Vec<(BoundarySite<G>, IncomingEdgeIndex)>,
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
    fn add_subgraphs(&mut self, graph: &PortDiffGraph<G>) {
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
    fn flatten_incoming_edges(&mut self, graph: &PortDiffGraph<G>) {
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

    /// Add a new boundary site at `site`, linked to the same parent port as `port`.
    fn append_boundary(&mut self, boundary: BoundarySite<G>, port: Owned<BoundaryIndex, G>) {
        let Owned { data: port, owner } = port;
        let edge_index = owner.incoming_edge_index(port).unwrap();
        let new_edge_index = self.edge_index_map[&(&owner).into()][&edge_index];

        // Add to boundary
        self.boundary.push((boundary, new_edge_index));

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

    /// Given a map from parent ports to boundary ports, find all boundary edges
    /// that need to be added.
    fn add_boundary_edges(
        &mut self,
        mut port_map: BTreeMap<Owned<BoundPort<G::Edge>, G>, BoundarySite<G>>,
    ) {
        // Find the maximum wire ID so we can initialize the UnionFind with the
        // correct capacity.
        let max_wire_id = port_map
            .values()
            .filter_map(|v| match *v {
                BoundarySite::Site(..) => None,
                BoundarySite::Wire { id, .. } => Some(id),
            })
            .max()
            .unwrap_or_default();
        let mut wires_uf = QuickUnionUf::<UnionBySize>::new(max_wire_id + 1);
        // Store for each wire its left/right ends (if they exist).
        let mut wires_opp_ends: Vec<[Option<Site<_, _>>; 2]> = vec![[None, None]; max_wire_id + 1];

        while let Some((parent_port, new_boundary)) = port_map.pop_first() {
            let parent_opp_port = parent_port.opposite();
            let new_opp_boundary = if let Some(new_opp_boundary) = port_map.remove(&parent_opp_port)
            {
                // The new edge is between two new sites
                new_opp_boundary
            } else {
                // Find (old) opposite site by following the edge in parent and
                // then translating to the new site with `node_map`
                self.map_site(parent_opp_port.site()).expect(
                    "a parent port was neither a boundary port nor a non-rewritten port in child",
                ).into()
            };
            let (left, right) = match parent_port.data.end {
                EdgeEnd::Left => (new_boundary, new_opp_boundary),
                EdgeEnd::Right => (new_opp_boundary, new_boundary),
            };
            match (left, right) {
                (BoundarySite::Site(left), BoundarySite::Site(right)) => {
                    self.graph.link_sites(left, right);
                }
                (BoundarySite::Site(left), BoundarySite::Wire { id, end }) => {
                    assert!(matches!(end, EdgeEnd::Right));
                    let entry = &mut wires_opp_ends[id][0];
                    assert!(entry.is_none(), "more than one value for same wire end");
                    *entry = Some(left);
                }
                (BoundarySite::Wire { id, end }, BoundarySite::Site(right)) => {
                    assert!(matches!(end, EdgeEnd::Left));
                    let entry = &mut wires_opp_ends[id][1];
                    assert!(entry.is_none(), "more than one value for same wire end");
                    *entry = Some(right);
                }
                (BoundarySite::Wire { id: id1, .. }, BoundarySite::Wire { id: id2, .. }) => {
                    wires_uf.union(id1, id2);
                }
            }
        }

        // The values of wires_opp_ends, but indexed at the root wires
        let mut wires_opp_ends_root = BTreeMap::new();
        for (i, sites) in wires_opp_ends.into_iter().enumerate() {
            let root = wires_uf.find(i);
            let root_site = wires_opp_ends_root.entry(root).or_insert([None, None]);
            for (s, ns) in sites.into_iter().zip(root_site.iter_mut()) {
                if let Some(s) = s {
                    assert!(ns.is_none(), "more than one value for same wire end");
                    *ns = Some(s);
                }
            }
        }

        // Link all wires endpoints
        for [left, right] in wires_opp_ends_root.values() {
            if let (Some(left), Some(right)) = (left, right) {
                self.graph.link_sites(left.clone(), right.clone());
            }
        }

        // Merge wire boundaries: if a port is connected to a wire, which itself
        // is connected to a boundary, then the boundary site must be replaced
        // to a concrete site instead of the wire.
        let boundary_wires = self
            .boundary
            .iter()
            .enumerate()
            .filter_map(|(i, (site, _))| match site {
                BoundarySite::Site(_) => None,
                &BoundarySite::Wire { end, id } => Some((i, id, end)),
            })
            .collect_vec(); // Tuples of boundary indices and wire ids + ends
        for (i, id, end) in boundary_wires {
            if id > max_wire_id {
                continue; // not a wire we know anything about
            }
            let id = wires_uf.find(id);
            let index = match end {
                EdgeEnd::Left => 0,
                EdgeEnd::Right => 1,
            };
            let Some(sites) = wires_opp_ends_root.get(&id) else {
                continue;
            };
            assert!(
                sites[1 - index].is_none(),
                "found both a boundary and internal edge at same port"
            );
            if let Some(site) = sites[index].as_ref() {
                // change away from wire to concrete site
                self.boundary[i].0 = BoundarySite::Site(site.clone());
            }
        }
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
}
