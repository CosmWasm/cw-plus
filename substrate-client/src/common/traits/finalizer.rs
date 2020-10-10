use crate::common::types::blockchain_result::BlockchainResult;
use sp_api::BlockId;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::Justification;

/// Finalize Facilities
pub trait Finalizer<Block: BlockT> {
    /// Finalize a block.
    ///
    /// This will implicitly finalize all blocks up to it and
    /// fire finality notifications.
    ///
    /// If the block being finalized is on a different fork from the current
    /// best block, the finalized block is set as best. This might be slightly
    /// inaccurate (i.e. outdated). Usages that require determining an accurate
    /// best block should use `SelectChain` instead of the client.
    ///
    /// Pass a flag to indicate whether finality notifications should be propagated.
    /// This is usually tied to some synchronization state, where we don't send notifications
    /// while performing major synchronization work.
    fn finalize_block(
        &self,
        id: BlockId<Block>,
        justification: Option<Justification>,
    ) -> BlockchainResult<()>;
}
