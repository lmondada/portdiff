use std::{
    cell::Ref,
    collections::BTreeSet,
    fmt::{self, Debug},
    rc::{Rc, Weak},
};

use crate::{EdgeEndType, Port};

use crate::PortDiff;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EdgeEnd {
    B(BoundaryEdge),
    I(InternalEdge, EdgeEndType),
}

impl EdgeEnd {
    pub(super) fn node_port<'d, V, P>(&'d self, diff: &'d PortDiff<V, P>) -> &'d Port<V, P> {
        match &self {
            EdgeEnd::B(b_edge) => &diff.boundary_edge(b_edge),
            EdgeEnd::I(i_edge, EdgeEndType::Left) => &diff.internal_edge(i_edge).left,
            EdgeEnd::I(i_edge, EdgeEndType::Right) => &diff.internal_edge(i_edge).right,
        }
    }

    pub(super) fn node<'d, V, P>(&'d self, diff: &'d PortDiff<V, P>) -> &'d V {
        &self.node_port(diff).node
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BoundaryEdge(pub(super) usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    owner: Rc<PortDiff<V, P>>,
    edge: InternalEdge,
    boundary_end: EdgeEndType,
    exclude_vertices: BTreeSet<V>,
}

impl<V, P> Debug for AncestorEdge<V, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AncestorEdge{{{}}}", self.edge.0)
    }
}

#[derive(Clone)]
pub(super) struct DescendantEdge<V, P> {
    owner: Weak<PortDiff<V, P>>,
    edge: BoundaryEdge,
    exclude_vertices: BTreeSet<V>,
}

impl<V, P> Debug for DescendantEdge<V, P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "DescendantEdge{{{}}}", self.edge.0)
    }
}

impl<V, P> AncestorEdge<V, P> {
    pub(super) fn new_internal_edge(edge: EdgeEnd, parent: &Rc<PortDiff<V, P>>) -> Self {
        let EdgeEnd::I(edge, boundary_end) = edge else {
            panic!("Expected internal edge");
        };
        Self {
            owner: parent.clone(),
            edge,
            boundary_end,
            exclude_vertices: BTreeSet::new(),
        }
    }

    pub(super) fn owner(&self) -> &Rc<PortDiff<V, P>> {
        &self.owner
    }

    pub(super) fn edge_end(&self) -> EdgeEnd {
        EdgeEnd::I(self.edge, self.boundary_end)
    }

    pub(super) fn exclude_vertices(&self) -> &BTreeSet<V> {
        &self.exclude_vertices
    }

    pub(super) fn opposite(&self) -> Self
    where
        V: Clone,
    {
        Self {
            owner: self.owner().clone(),
            edge: self.edge,
            boundary_end: self.boundary_end.opposite(),
            exclude_vertices: self.exclude_vertices.clone(),
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
        self.exclude_vertices()
            .intersection(&other.exclude_vertices())
            .next()
            .is_none()
    }
}

impl<V, P> DescendantEdge<V, P> {
    pub(super) fn new(
        owner: &Rc<PortDiff<V, P>>,
        edge: BoundaryEdge,
        exclude_vertices: BTreeSet<V>,
    ) -> Self {
        Self {
            owner: Rc::downgrade(owner),
            edge,
            exclude_vertices,
        }
    }

    pub(super) fn edge_end(&self) -> EdgeEnd {
        EdgeEnd::B(self.edge)
    }

    pub(super) fn exclude_vertices(&self) -> &BTreeSet<V> {
        &self.exclude_vertices
    }

    pub(super) fn owner(&self) -> Option<Rc<PortDiff<V, P>>> {
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
