use uuid::Uuid;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniqueVertex(Uuid);

static mut COUNTER: u128 = 0;

impl UniqueVertex {
    /// TODO: this should be random
    pub fn new_unsafe() -> Self {
        let id = unsafe {
            COUNTER += 1;
            Uuid::from_u128(COUNTER)
        };
        Self::from_id(id)
    }

    pub fn id(&self) -> Uuid {
        self.0
    }

    pub fn from_id(id: Uuid) -> Self {
        Self(id)
    }
}
