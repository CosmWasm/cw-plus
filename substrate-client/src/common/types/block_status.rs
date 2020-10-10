/// Block status.
#[derive(Debug, PartialEq, Eq)]
pub enum BlockStatus {
    /// Already in the blockchain.
    InChain,
    /// Not in the queue or the blockchain.
    Unknown,
}
