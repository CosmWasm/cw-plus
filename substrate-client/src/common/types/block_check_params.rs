use sp_runtime::traits::Block as BlockT;
use sp_runtime::traits::NumberFor;

/// Data required to check validity of a Block.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BlockCheckParams<Block: BlockT> {
    /// Hash of the block that we verify.
    pub hash: Block::Hash,
    /// Block number of the block that we verify.
    pub number: NumberFor<Block>,
    /// Parent hash of the block that we verify.
    pub parent_hash: Block::Hash,
    /// Allow importing the block skipping state verification if parent state is missing.
    pub allow_missing_state: bool,
    /// Re-validate existing block.
    pub import_existing: bool,
}
