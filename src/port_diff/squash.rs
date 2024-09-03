use std::collections::{BTreeMap, BTreeSet};

use bimap::BiBTreeMap;
use derive_more::From;
use derive_where::derive_where;
use petgraph::visit::{EdgeRef, IntoEdges};
use union_find::{QuickUnionUf, UnionBySize, UnionFind};

use crate::{
    port::{BoundPort, BoundaryIndex, EdgeEnd, Port, Site},
    Graph, GraphView, NodeId, PortDiff,
};

use super::{BoundaryPort, EdgeData, IncomingEdgeIndex, Owned, PortDiffData};

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
                let old_boundary = diff.boundary_port(bd_index);
                let new_boundary = match old_boundary {
                    BoundaryPort::Site(site) => {
                        if !builder.contains(Owned::new(site.node, diff.clone())) {
                            // Site is outside of the rewritten region.
                            continue;
                        }
                        builder
                            .map_site(Owned::new(site.clone(), diff.clone()))
                            .unwrap()
                            .into()
                    }
                    sentinel @ BoundaryPort::Sentinel(_) => sentinel.clone(),
                };

                match try_resolve_port(Owned::new(bd_index, diff.clone()), &all_nodes) {
                    Ok(bound_port) => {
                        resolved_ports_map.insert(bound_port, new_boundary);
                    }
                    Err(boundary) => {
                        builder.append_boundary(new_boundary, boundary);
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
    boundary: Vec<(BoundaryPort<G>, IncomingEdgeIndex)>,
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

    /// Add a new boundary site at `site`, linked to the same parent port as `port`.
    fn append_boundary(&mut self, boundary: BoundaryPort<G>, port: Owned<BoundaryIndex, G>) {
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
        mut port_map: BTreeMap<Owned<BoundPort<G::Edge>, G>, BoundaryPort<G>>,
    ) {
        // Max capacity: worst case every boundar port maps to a non-boundary port
        let mut uf = PortUF::with_capacity(port_map.len() * 2);

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
            match parent_port.data.end {
                EdgeEnd::Left => uf.union(new_boundary, new_opp_boundary),
                EdgeEnd::Right => uf.union(new_opp_boundary, new_boundary),
            }
        }

        for (site1, site2) in uf.into_pairs() {
            self.graph.link_sites(site1, site2);
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

    /// Whether `node` is also in the new replacement graph.
    fn contains(&self, node: Owned<G::Node, G>) -> bool {
        let Some(diff_nodes) = self.nodes_map.get(&(&node.owner).into()) else {
            return false;
        };
        diff_nodes.contains_key(&node.data)
    }
}

/// Union-find data structure for gathering all boundary ports (sentinels).
///
/// At the end of the construction, each equivalence class should have exactly
/// two non-sentinel elements.
struct PortUF<G: Graph> {
    uf_array: QuickUnionUf<UnionBySize>,
    port_map: BiBTreeMap<UFItem<G>, usize>,
    len: usize,
}

impl<G: Graph> PortUF<G> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            uf_array: QuickUnionUf::new(capacity),
            port_map: BiBTreeMap::default(),
            len: 0,
        }
    }

    fn get_index(&mut self, item: &UFItem<G>) -> usize {
        if let Some(&index) = self.port_map.get_by_left(item) {
            index
        } else {
            let new_index = self.len;
            self.port_map.insert(item.clone(), new_index);
            self.len += 1;
            new_index
        }
    }

    fn union(&mut self, left: BoundaryPort<G>, right: BoundaryPort<G>) {
        let left = match left {
            BoundaryPort::Site(site) => UFItem::Site(site, EdgeEnd::Left),
            BoundaryPort::Sentinel(sentinel) => UFItem::Sentinel(sentinel),
        };
        let right = match right {
            BoundaryPort::Site(site) => UFItem::Site(site, EdgeEnd::Right),
            BoundaryPort::Sentinel(sentinel) => UFItem::Sentinel(sentinel),
        };
        let left_index = self.get_index(&left);
        let right_index = self.get_index(&right);
        self.uf_array.union(left_index, right_index);
    }

    fn into_pairs(
        mut self,
    ) -> impl Iterator<Item = (Site<G::Node, G::PortLabel>, Site<G::Node, G::PortLabel>)> {
        let mut classes: BTreeMap<usize, Vec<(Site<G::Node, G::PortLabel>, EdgeEnd)>> =
            BTreeMap::new();
        for (port, i) in self.port_map.into_iter() {
            let UFItem::Site(site, end) = port else {
                continue;
            };
            let class = self.uf_array.find(i);
            classes.entry(class).or_default().push((site, end));
        }
        classes.into_iter().map(|(_, mut sites)| {
            assert_eq!(sites.len(), 2, "invalid sentinels");
            let (site1, end1) = sites.remove(0);
            let (site2, end2) = sites.remove(0);
            match (end1, end2) {
                (EdgeEnd::Left, EdgeEnd::Right) => (site1, site2),
                (EdgeEnd::Right, EdgeEnd::Left) => (site2, site1),
                _ => panic!("invalid sentinel pair"),
            }
        })
    }
}

#[derive(Debug, Clone, Copy, From)]
#[derive_where(PartialEq, Eq, PartialOrd, Ord)]
enum UFItem<G: Graph> {
    Sentinel(usize),
    Site(Site<G::Node, G::PortLabel>, EdgeEnd),
}
