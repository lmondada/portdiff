#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PortDiffId(pub(crate) String);

#[derive(Default)]
pub(crate) struct PortDiffIdCreator {
    max_ind: usize,
}

impl PortDiffIdCreator {
    pub fn create(&mut self) -> PortDiffId {
        let id = format!("PORTDIFF_{}", self.max_ind);
        self.max_ind += 1;
        PortDiffId(id)
    }
}
