use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct UniqueVertex(Uuid);

impl UniqueVertex {
    pub(crate) fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
