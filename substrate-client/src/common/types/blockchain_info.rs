use sp_runtime::traits::{Block as BlockT, Header as HeaderT};

/// Blockchain info
#[derive(Debug)]
pub struct BlockchainInfo<Block: BlockT> {
    /// Best block hash.
    pub best_hash: Block::Hash,
    /// Best block number.
    pub best_number: <<Block as BlockT>::Header as HeaderT>::Number,
    /// Genesis block hash.
    pub genesis_hash: Block::Hash,
    /// The head of the finalized chain.
    pub finalized_hash: Block::Hash,
    /// Last finalized block number.
    pub finalized_number: <<Block as BlockT>::Header as HeaderT>::Number,
    /// Number of concurrent leave forks.
    pub number_leaves: usize,
}
