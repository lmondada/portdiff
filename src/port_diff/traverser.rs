use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;

use crate::{
    port::{Port, Site},
    Graph, PortDiff,
};

use super::{EdgeData, PortDiffPtr, UniqueNodeId};

impl<G: Graph> PortDiff<G> {
    fn all_ports<'a>(
        &'a self,
        unbound_port: &'a Site<G::Node, G::PortLabel>,
    ) -> impl Iterator<Item = Port<G>> + 'a {
        let internal = self
            .graph
            .get_bound_ports(unbound_port)
            .map(|port| Port::Bound {
                port,
                owner: self.clone(),
            });
        let boundary = self
            .boundary
            .contains_key(unbound_port)
            .then_some(Port::Unbound {
                port: unbound_port.clone(),
                owner: self.clone(),
            });
        internal.chain(boundary)
    }
}

pub struct DiffTraverser<G: Graph> {
    exclude_nodes: BTreeMap<PortDiffPtr<G>, BTreeSet<G::Node>>,
}

impl<G: Graph> Clone for DiffTraverser<G> {
    fn clone(&self) -> Self {
        Self {
            exclude_nodes: self.exclude_nodes.clone(),
        }
    }
}

impl<G: Graph> Default for DiffTraverser<G> {
    fn default() -> Self {
        Self {
            exclude_nodes: BTreeMap::new(),
        }
    }
}

impl<G: Graph> PortDiff<G> {
    pub fn traverse<'a>(
        node: UniqueNodeId<G>,
        port_label: G::PortLabel,
        known_edges: &[EdgeData<G>],
    ) -> Vec<EdgeData<G>>
    where
        G: 'a,
    {
        let diff = node.owner;
        let unbound_port = Site {
            node: node.node,
            port: port_label,
        };
        diff.all_ports(&unbound_port)
            .flat_map(|port| PortDiff::opposite_ports(port))
            // TODO: This should be awfully slow
            .filter(|edge| {
                PortDiff::are_compatible(edge_diffs(known_edges.iter().chain(Some(edge))))
            })
            .collect_vec()
    }
}

fn edge_diffs<'d, G: Graph>(
    edges: impl IntoIterator<Item = &'d EdgeData<G>>,
) -> impl Iterator<Item = &'d PortDiff<G>>
where
    G: 'd,
{
    edges
        .into_iter()
        .flat_map(|edge| match edge {
            EdgeData::Internal { owner, .. } => vec![owner],
            EdgeData::Boundary { left, right } => vec![left.owner(), right.owner()],
        })
        .unique()
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::port_diff::tests::{root_diff, TestPortDiff};

    use super::*;

    #[rstest]
    fn test_traverse(root_diff: TestPortDiff) {
        let (n0, n1, n2, n3) = root_diff.nodes().collect_tuple().unwrap();
        let child_a = root_diff.identity_subgraph([n0, n1]);
        let child_b = root_diff.identity_subgraph([n2, n3]);

        let edges = child_a.traverse(n0, 0, &[]);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].right().node(), n1);
    }
}
