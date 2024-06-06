use std::collections::BTreeMap;

use crate::{Port, PortDiff, PortEdge, UniqueVertex};

type V = UniqueVertex;

impl<P: Clone + Ord> PortDiff<V, P> {
    /// Replace the internal edges with `new_edges`.
    ///
    /// All vertices with a changed set of ports are replaced with new vertices.
    ///
    /// Optionally, indicate new boundary ports. The set of new boundary ports
    /// must be of the same length. If a boundary port is `None`, the existing
    /// boundary port is retained.
    pub fn rewrite(
        &self,
        new_edges: &[PortEdge<V, P>],
        new_boundary_ports: Vec<Option<Port<V, P>>>,
    ) -> Result<Self, String> {
        if new_boundary_ports.len() != self.boundary_ports.len() {
            return Err("Mismatching number of boundary ports".to_string());
        }

        // Get port sets to compare
        let new_ports = get_port_lists(new_edges);
        let curr_ports = get_port_lists(self.edges.iter());

        // Create the new vertices where required
        let mut new_vertices = BTreeMap::<V, V>::new();
        for (&curr_v, curr_ports) in curr_ports.iter() {
            let new_ports = new_ports.get(&curr_v);
            if Some(curr_ports) != new_ports {
                let new_v = UniqueVertex::new();
                new_vertices.insert(curr_v, new_v);
            }
        }

        // Lift the vertex name change to ports
        let new_port = |Port { node, port }: &Port<_, P>| {
            let &node = new_vertices.get(node).unwrap_or(node);
            Port {
                node,
                port: port.clone(),
            }
        };

        // Create the new edges
        let new_edges = new_edges.iter().map(|e| PortEdge {
            left: new_port(&e.left),
            right: new_port(&e.right),
        });

        // Create new boundary ports
        let boundary_ports = self
            .boundary_ports
            .iter()
            .zip(&new_boundary_ports)
            .map(|(p, new_p)| new_p.as_ref().unwrap_or(p))
            .map(new_port)
            .collect();

        Ok(Self {
            edges: new_edges.collect(),
            boundary_ports,
            boundary_anc: self.boundary_anc.clone(),
            boundary_desc: self.boundary_desc.clone(),
        })
    }
}

type PortLists<P> = BTreeMap<V, Vec<P>>;

fn get_port_lists<'e, P: Ord + Clone + 'e>(
    edges: impl IntoIterator<Item = &'e PortEdge<V, P>>,
) -> PortLists<P> {
    let mut adjacency = PortLists::new();
    for PortEdge { left, right } in edges {
        for src in [left, right] {
            adjacency
                .entry(src.node)
                .or_default()
                .push(src.port.clone());
        }
    }
    for adj_list in adjacency.values_mut() {
        adj_list.sort_unstable();
    }
    adjacency
}
