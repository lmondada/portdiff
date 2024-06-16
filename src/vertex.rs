use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniqueVertex(Uuid);

/// Vertex IDs that are globally unique, using UUID.
impl UniqueVertex {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn id(&self) -> Uuid {
        self.0
    }

    pub fn from_id(id: Uuid) -> Self {
        Self(id)
    }
}

/// Deterministic Vertex IDs to be used for testing.
#[derive(Debug, Default, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DetVertex(pub String);

/// Create vertices in a deterministic order (incrementing usize).
#[derive(Debug, Default, Clone)]
pub struct DetVertexCreator {
    pub max_ind: usize,
}

impl DetVertex {
    pub fn id(&self) -> &str {
        &self.0
    }
}

impl DetVertexCreator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self) -> DetVertex {
        let res = DetVertex(format!("{}", self.max_ind));
        self.max_ind += 1;
        res
    }
}
