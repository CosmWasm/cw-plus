use crate::common::types::block_status::BlockStatus;
use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::blockchain_info::BlockchainInfo;
use crate::common::types::blockchain_result::BlockchainResult;
use sp_api::BlockId;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};

/// Blockchain database header backend. Does not perform any validation.
pub trait HeaderBackend<Block: BlockT>: Send + Sync {
    /// Get block header. Returns `None` if block is not found.
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>>;
    /// Get blockchain info.
    fn info(&self) -> BlockchainInfo<Block>;
    /// Get block status.
    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus>;
    /// Get block number by hash. Returns `None` if the header is not in the chain.
    fn number(
        &self,
        hash: Block::Hash,
    ) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>>;
    /// Get block hash by number. Returns `None` if the header is not in the chain.
    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>>;

    /// Convert an arbitrary block ID into a block hash.
    fn block_hash_from_id(&self, id: &BlockId<Block>) -> BlockchainResult<Option<Block::Hash>> {
        match *id {
            BlockId::Hash(h) => Ok(Some(h)),
            BlockId::Number(n) => self.hash(n),
        }
    }

    /// Convert an arbitrary block ID into a block hash.
    fn block_number_from_id(
        &self,
        id: &BlockId<Block>,
    ) -> BlockchainResult<Option<NumberFor<Block>>> {
        match *id {
            BlockId::Hash(_) => Ok(self.header(*id)?.map(|h| h.number().clone())),
            BlockId::Number(n) => Ok(Some(n)),
        }
    }

    /// Get block header. Returns `UnknownBlock` error if block is not found.
    fn expect_header(&self, id: BlockId<Block>) -> BlockchainResult<Block::Header> {
        self.header(id)?
            .ok_or_else(|| BlockchainError::UnknownBlock(format!("Expect header: {}", id)))
    }

    /// Convert an arbitrary block ID into a block number. Returns `UnknownBlock` error if block is not found.
    fn expect_block_number_from_id(
        &self,
        id: &BlockId<Block>,
    ) -> BlockchainResult<NumberFor<Block>> {
        self.block_number_from_id(id).and_then(|n| {
            n.ok_or_else(|| {
                BlockchainError::UnknownBlock(format!("Expect block number from id: {}", id))
            })
        })
    }

    /// Convert an arbitrary block ID into a block hash. Returns `UnknownBlock` error if block is not found.
    fn expect_block_hash_from_id(&self, id: &BlockId<Block>) -> BlockchainResult<Block::Hash> {
        self.block_hash_from_id(id).and_then(|n| {
            n.ok_or_else(|| {
                BlockchainError::UnknownBlock(format!("Expect block hash from id: {}", id))
            })
        })
    }
}
