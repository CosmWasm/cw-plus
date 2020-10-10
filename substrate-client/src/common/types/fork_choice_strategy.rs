/// Fork choice strategy.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ForkChoiceStrategy {
    /// Longest chain fork choice.
    LongestChain,
    /// Custom fork choice rule, where true indicates the new block should be the best block.
    Custom(bool),
}
