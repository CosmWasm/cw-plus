/// State of a new block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewBlockState {
    /// Normal block.
    Normal,
    /// New best block.
    Best,
    /// Newly finalized block (implicitly best).
    Final,
}

impl NewBlockState {
    pub fn is_best(&self) -> bool {
        *self == Self::Best
    }
}
