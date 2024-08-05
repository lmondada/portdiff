use std::collections::{BTreeMap, BTreeSet};

use relrc::RelRc;

use crate::{
    port::{BoundaryIndex, EdgeEnd, Port, Site},
    Graph, PortDiff,
};

use super::{EdgeData, IncomingEdgeIndex, Owned, PortDiffData, PortDiffPtr};

impl<G: Graph> PortDiff<G> {
    /// Squash `self` into its parents, by merging two rewrites into one.
    ///
    /// The new diff has the grandparents of `self` as parents and extracting
    /// an output from the new diff is equivalent to extracting a diff from `self`.
    pub fn squash(&self) -> Self {
        let mut builder = Builder::new(self.graph.clone());

        // For each incoming edge, add the subgraph at the parent (minus the
        // nodes removed by the rewrite).
        builder.add_parent_subgraphs(self.all_incoming());

        builder.flatten_incoming_edges(self.all_parents());

        // For each boundary port of `self`, consider the port in one of the parents
        // it maps to and do one of
        //  - if parent port is a boundary port, then add to new boundary
        //  - if parent port is a bound port, then add an edge from the child
        //    port to the other port it is connected to (either in the parent or child)
        let mut ports_map = BTreeMap::new();

        for self_index in self.boundary_iter() {
            match self.parent_port(self_index) {
                Owned {
                    data: Port::Boundary(port),
                    owner,
                } => {
                    let site = owner.boundary_site(port).clone();
                    builder.append_boundary(site, Owned { data: port, owner });
                }
                Owned {
                    data: Port::Bound(port),
                    owner,
                } => {
                    // We delay inserting the new edges until we have traversed
                    // all boundaries. This makes finding the correct sites for
                    // for the new edges easier
                    ports_map.insert(
                        Owned { data: port, owner },
                        self.boundary_site(self_index).clone(),
                    );
                }
            }
        }
        // Insert new edges
        while let Some((parent_port, new_site)) = ports_map.pop_first() {
            let parent_opp_port = parent_port.opposite();
            let new_opp_site = if let Some(new_opp_site) = ports_map.remove(&parent_opp_port) {
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

        // Finally add to the boundary the boundary ports of the parents on
        // non-rewritten nodes
        for parent in self.all_parents() {
            for index in parent.boundary_iter() {
                let site = parent.boundary_site(index).clone();
                if let Some(new_site) = builder.map_site(Owned {
                    data: site,
                    owner: parent.clone(),
                }) {
                    builder.append_boundary(
                        new_site,
                        Owned {
                            data: index,
                            owner: parent.clone(),
                        },
                    )
                }
            }
        }
        builder.finish()
    }
}

struct Builder<G: Graph> {
    /// The new boundary
    boundary: Vec<(Site<G::Node, G::PortLabel>, IncomingEdgeIndex)>,
    /// The new incoming edges and their parent
    incoming_edges: Vec<(PortDiff<G>, EdgeData<G>)>,
    /// For each parent, a map from the old edge index to the new edge index
    edge_index_map: BTreeMap<PortDiffPtr<G>, BTreeMap<IncomingEdgeIndex, IncomingEdgeIndex>>,
    /// For each parent, a map from the old node to the new node
    nodes_map: BTreeMap<PortDiffPtr<G>, BTreeMap<G::Node, G::Node>>,
    /// The new replacement graph
    graph: G,
}

impl<G: Graph> Builder<G> {
    fn new(graph: G) -> Self {
        Self {
            boundary: vec![],
            incoming_edges: vec![],
            edge_index_map: BTreeMap::new(),
            nodes_map: BTreeMap::new(),
            graph,
        }
    }

    /// For the source nodes of `all_incoming`, add the subgraphs of the non-rewritten
    /// nodes.
    ///
    /// For each parent node, return a map from nodes in the parent graph to nodes
    /// in `graph`.
    fn add_parent_subgraphs<'a>(
        &mut self,
        all_incoming: impl IntoIterator<
            Item = &'a relrc::edge::InnerEdgeData<PortDiffData<G>, EdgeData<G>>,
        >,
    ) where
        G: 'a,
    {
        // The list of unique parents
        let mut parents = vec![];
        // Map parent pointers to the set of nodes to keep (i.e. nodes not
        // in any of the edges to `self`)
        let mut parents_nodes = BTreeMap::new();

        for edge in all_incoming {
            let parent = edge.source();
            let parent_ptr = RelRc::as_ptr(parent);
            if !parents_nodes.contains_key(&parent_ptr) {
                parents_nodes.insert(
                    parent_ptr,
                    parent.value().graph.nodes_iter().collect::<BTreeSet<_>>(),
                );
                parents.push(parent);
            }
            let parents_nodes = parents_nodes.get_mut(&parent_ptr).unwrap();
            for node in edge.value().subgraph.nodes() {
                parents_nodes.remove(&node);
            }
        }

        for parent in parents {
            let nodes = &parents_nodes[&RelRc::as_ptr(&parent)];
            let nodes_map = self.graph.add_subgraph(&parent.value().graph, nodes);
            self.nodes_map.insert(RelRc::as_ptr(&parent), nodes_map);
        }
    }

    /// Flatten the incoming edges of the `diffs` into a single list of edges.
    ///
    /// Store a map from the old edge indices to the new edge indices.
    fn flatten_incoming_edges<'a>(&mut self, diffs: impl IntoIterator<Item = PortDiff<G>>)
    where
        G: 'a,
    {
        for diff in diffs {
            let mut edge_index_map = BTreeMap::new();
            for (index, edge) in diff.all_incoming().iter().enumerate() {
                let new_index = self.incoming_edges.len();
                self.incoming_edges.push((
                    edge.source().clone().into(),
                    EdgeData {
                        subgraph: edge.value().subgraph.clone(),
                        port_map: Default::default(),
                    },
                ));
                edge_index_map.insert(IncomingEdgeIndex(index), IncomingEdgeIndex(new_index));
            }
            self.edge_index_map
                .insert(PortDiff::as_ptr(&diff), edge_index_map);
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
        let new_edge_index = self.edge_index_map[&PortDiff::as_ptr(&owner)][&edge_index];

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
        site.filter_map_node(|n| {
            self.nodes_map
                .get(&PortDiff::as_ptr(&owner))?
                .get(&n)
                .copied()
        })
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
