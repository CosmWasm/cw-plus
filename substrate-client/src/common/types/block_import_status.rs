/// Block status.
#[derive(Debug, PartialEq, Eq)]
pub enum BlockImportStatus {
    /// Added to the import queue.
    Queued,
    /// Already in the blockchain and the state is available.
    InChainWithState,
    /// In the blockchain, but the state is not available.
    InChainPruned,
    /// Block or parent is known to be bad.
    KnownBad,
    /// Not in the queue or the blockchain.
    Unknown,
}
