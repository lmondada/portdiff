use std::{
    cell::Ref,
    collections::BTreeSet,
    fmt::{self, Debug},
};

use crate::{port_diff::WeakPortDiff, EdgeEndType};

use crate::PortDiff;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum EdgeEnd {
    B(BoundaryEdge),
    I(InternalEdge, EdgeEndType),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct BoundaryEdge(pub(super) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct InternalEdge(pub(super) usize);

impl InternalEdge {
    pub(super) fn to_left_end(self) -> EdgeEnd {
        EdgeEnd::I(self, EdgeEndType::Left)
    }

    pub(super) fn to_right_end(self) -> EdgeEnd {
        EdgeEnd::I(self, EdgeEndType::Right)
    }
}

#[derive(Clone)]
pub(super) struct AncestorEdge<V, P> {
    owner: PortDiff<V, P>,
    edge: InternalEdge,
    boundary_end: EdgeEndType,
    used_vertices: BTreeSet<V>,
}

impl<V, P> Debug for AncestorEdge<V, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AncestorEdge{{{}}}", self.edge.0)
    }
}

#[derive(Clone)]
pub(super) struct DescendantEdge<V, P> {
    owner: WeakPortDiff<V, P>,
    edge: BoundaryEdge,
    used_vertices: BTreeSet<V>,
}

impl<V, P> Debug for DescendantEdge<V, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "DescendantEdge{{{}}} @ {:?}",
            self.edge.0,
            self.owner.upgrade().map(|x| x.as_ptr())
        )
    }
}

impl<V, P> AncestorEdge<V, P> {
    pub(super) fn new_internal_edge(edge: EdgeEnd, parent: &PortDiff<V, P>) -> Self {
        let EdgeEnd::I(edge, boundary_end) = edge else {
            panic!("Expected internal edge");
        };
        Self {
            owner: parent.clone(),
            edge,
            boundary_end,
            used_vertices: BTreeSet::new(),
        }
    }

    pub(super) fn owner(&self) -> &PortDiff<V, P> {
        &self.owner
    }

    pub(super) fn edge_end(&self) -> EdgeEnd {
        EdgeEnd::I(self.edge, self.boundary_end)
    }

    pub(super) fn internal_edge(&self) -> InternalEdge {
        self.edge
    }

    pub(super) fn edge_end_type(&self) -> EdgeEndType {
        self.boundary_end
    }

    pub(super) fn used_vertices(&self) -> &BTreeSet<V> {
        &self.used_vertices
    }

    pub(super) fn add_used_vertices(&mut self, vertices: impl IntoIterator<Item = V>)
    where
        V: Ord + Eq,
    {
        self.used_vertices.extend(vertices)
    }

    pub(super) fn opposite(&self) -> Self
    where
        V: Clone,
    {
        Self {
            owner: self.owner().clone(),
            edge: self.edge,
            boundary_end: self.boundary_end.opposite(),
            used_vertices: self.used_vertices.clone(),
        }
    }

    pub(super) fn get_descendant_edges(&self) -> Ref<[DescendantEdge<V, P>]> {
        self.owner()
            .get_descendant_edges(&self.edge, self.boundary_end)
    }

    pub(super) fn is_compatible(&self, other: &DescendantEdge<V, P>) -> bool
    where
        V: Ord,
    {
        self.used_vertices()
            .intersection(&other.used_vertices())
            .next()
            .is_none()
    }
}

impl<V, P> DescendantEdge<V, P> {
    pub(super) fn new(
        owner: &PortDiff<V, P>,
        edge: BoundaryEdge,
        used_vertices: BTreeSet<V>,
    ) -> Self {
        Self {
            owner: owner.downgrade(),
            edge,
            used_vertices,
        }
    }

    pub(super) fn edge_end(&self) -> EdgeEnd {
        EdgeEnd::B(self.edge)
    }

    pub(super) fn boundary_edge(&self) -> BoundaryEdge {
        self.edge
    }

    pub(super) fn used_vertices(&self) -> &BTreeSet<V> {
        &self.used_vertices
    }

    pub(super) fn owner(&self) -> Option<PortDiff<V, P>> {
        self.owner.upgrade()
    }
}

#[derive(Clone, Debug)]
pub(super) struct DescendantEdges<V, P> {
    pub(super) left: Vec<DescendantEdge<V, P>>,
    pub(super) right: Vec<DescendantEdge<V, P>>,
}

impl<V, P> Default for DescendantEdges<V, P> {
    fn default() -> Self {
        Self {
            left: vec![],
            right: vec![],
        }
    }
}

impl<V, P> DescendantEdges<V, P> {
    pub(super) fn remove_empty_refs(&mut self) {
        self.left.retain(|e| e.owner.upgrade().is_some());
        self.right.retain(|e| e.owner.upgrade().is_some());
    }

    pub(super) fn is_empty(&self) -> bool {
        self.left.is_empty() && self.right.is_empty()
    }
}
