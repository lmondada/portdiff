use std::{cell::RefCell, collections::BTreeMap};

use itertools::Itertools;

use crate::{edges::DescendantEdges, Port, PortDiff, PortEdge, UniqueVertex};

use super::PortDiffData;

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
        if new_boundary_ports.len() != self.data.boundary_ports.len() {
            return Err("Mismatching number of boundary ports".to_string());
        }
        if self.has_any_descendants() {
            return Err("Cannot rewrite port diff with descendants".to_string());
        }

        // Get port sets to compare
        let new_ports = get_port_lists(new_edges);
        let curr_ports = get_port_lists(&self.data.edges);

        // Create the new vertices where required
        let mut new_vertices = BTreeMap::<V, V>::new();
        for (&curr_v, curr_ports) in curr_ports.iter() {
            let new_ports = new_ports.get(&curr_v);
            if Some(curr_ports) != new_ports {
                let new_v = UniqueVertex::new_unsafe();
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
        let new_edges = new_edges
            .iter()
            .map(|e| PortEdge {
                left: new_port(&e.left),
                right: new_port(&e.right),
            })
            .collect_vec();
        let boundary_desc = RefCell::new(vec![DescendantEdges::default(); new_edges.len()]);

        // Create new boundary ports
        let boundary_ports = self
            .data
            .boundary_ports
            .iter()
            .zip(&new_boundary_ports)
            .map(|(p, new_p)| new_p.as_ref().unwrap_or(p))
            .map(new_port)
            .collect();

        // Add the removed vertices to `used_vertices` of ancestor edges
        let mut boundary_anc = self.data.boundary_anc.clone();
        for anc in &mut boundary_anc {
            anc.add_used_vertices(new_vertices.keys().cloned());
        }

        Ok(Self::new(PortDiffData {
            edges: new_edges,
            boundary_ports,
            boundary_anc,
            boundary_desc,
        }))
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::{
        edges::InternalEdge,
        port_diff::tests::{root_diff, test_nodes, TestPortDiff},
    };

    use super::*;

    #[rstest]
    fn test_rewrite(root_diff: TestPortDiff) {
        let nodes = test_nodes();
        let child = PortDiff::with_nodes([nodes[0], nodes[1]], &root_diff);
        let grandchild = child
            .rewrite(&[], vec![None; child.n_boundary_edges()])
            .unwrap();
        assert_eq!(grandchild.n_internal_edges(), 0);
        assert_eq!(grandchild.n_boundary_edges(), 1);
        dbg!(&root_diff);
        assert_eq!(
            root_diff
                .get_descendant_edges(&InternalEdge(3), crate::EdgeEndType::Left)
                .len(),
            2
        );
    }
}
