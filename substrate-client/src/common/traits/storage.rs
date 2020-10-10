use crate::common::traits::aux_store::AuxStore;
use crate::common::traits::header_backend::HeaderBackend;
use crate::common::traits::header_metadata::HeaderMetadata;
use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::blockchain_result::BlockchainResult;
use crate::common::types::new_block_state::NewBlockState;
use sp_api::BlockId;
use sp_runtime::traits::Block as BlockT;

/// Light client blockchain storage.
pub trait Storage<Block: BlockT>:
    AuxStore + HeaderBackend<Block> + HeaderMetadata<Block, Error = BlockchainError>
{
    /// Store new header. Should refuse to revert any finalized blocks.
    ///
    /// Takes new authorities, the leaf state of the new block, and
    /// any auxiliary storage updates to place in the same operation.
    fn import_header(&self, header: Block::Header, state: NewBlockState) -> BlockchainResult<()>;

    /// Set an existing block as new best block.
    fn set_head(&self, block: BlockId<Block>) -> BlockchainResult<()>;

    /// Mark historic header as finalized.
    fn finalize_header(&self, block: BlockId<Block>) -> BlockchainResult<()>;

    /// Get last finalized header.
    fn last_finalized(&self) -> BlockchainResult<Block::Hash>;
}
