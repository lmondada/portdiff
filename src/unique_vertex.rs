use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniqueVertex(Uuid);

impl UniqueVertex {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn id(&self) -> Uuid {
        self.0
    }
}
