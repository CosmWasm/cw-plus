use crate::common::traits::block_import::BlockImport;
use crate::common::traits::finalizer::Finalizer;
use crate::common::traits::header_backend::HeaderBackend;
use crate::common::traits::header_metadata::HeaderMetadata;
use crate::common::traits::storage::Storage;
use crate::common::types::block_check_params::BlockCheckParams;
use crate::common::types::block_import_params::BlockImportParams;
use crate::common::types::block_import_status::BlockImportStatus as ImportBlockStatus;
use crate::common::types::block_status::BlockStatus;
use crate::common::types::blockchain_error::BlockchainError;
use crate::common::types::blockchain_info::BlockchainInfo;
use crate::common::types::blockchain_result::BlockchainResult;
use crate::common::types::cached_header_metadata::CachedHeaderMetadata;
use crate::common::types::consensus_error::ConsensusError;
use crate::common::types::import_result::ImportResult;
use crate::common::types::new_block_state::NewBlockState;
use parity_scale_codec::alloc::sync::Arc;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::{Block as BlockT, Header as HeaderT, NumberFor};

/// Hash and number of a block.
#[derive(Debug, Clone)]
pub struct HashAndNumber<Block: BlockT> {
    /// The number of the block.
    pub number: NumberFor<Block>,
    /// The hash of the block.
    pub hash: Block::Hash,
}

/// A tree-route from one block to another in the chain.
///
/// All blocks prior to the pivot in the deque is the reverse-order unique ancestry
/// of the first block, the block at the pivot index is the common ancestor,
/// and all blocks after the pivot is the ancestry of the second block, in
/// order.
///
/// The ancestry sets will include the given blocks, and thus the tree-route is
/// never empty.
///
/// ```text
/// Tree route from R1 to E2. Retracted is [R1, R2, R3], Common is C, enacted [E1, E2]
///   <- R3 <- R2 <- R1
///  /
/// C
///  \-> E1 -> E2
/// ```
///
/// ```text
/// Tree route from C to E2. Retracted empty. Common is C, enacted [E1, E2]
/// C -> E1 -> E2
/// ```
#[derive(Debug)]
pub struct TreeRoute<Block: BlockT> {
    route: Vec<HashAndNumber<Block>>,
    pivot: usize,
}

impl<Block: BlockT> TreeRoute<Block> {
    /// Get a slice of all retracted blocks in reverse order (towards common ancestor)
    pub fn retracted(&self) -> &[HashAndNumber<Block>] {
        &self.route[..self.pivot]
    }

    /// Get the common ancestor block. This might be one of the two blocks of the
    /// route.
    pub fn common_block(&self) -> &HashAndNumber<Block> {
        self.route.get(self.pivot).expect(
            "tree-routes are computed between blocks; \
			which are included in the route; \
			thus it is never empty; qed",
        )
    }

    /// Get a slice of enacted blocks (descendents of the common ancestor)
    pub fn enacted(&self) -> &[HashAndNumber<Block>] {
        &self.route[self.pivot + 1..]
    }
}

/// Compute a tree-route between two blocks. See tree-route docs for more details.
fn tree_route<Block: BlockT, T: HeaderMetadata<Block>>(
    backend: &T,
    from: Block::Hash,
    to: Block::Hash,
) -> Result<TreeRoute<Block>, T::Error> {
    let mut from = backend.header_metadata(from)?;
    let mut to = backend.header_metadata(to)?;

    let mut from_branch = Vec::new();
    let mut to_branch = Vec::new();

    while to.number > from.number {
        to_branch.push(HashAndNumber {
            number: to.number,
            hash: to.hash,
        });

        to = backend.header_metadata(to.parent)?;
    }

    while from.number > to.number {
        from_branch.push(HashAndNumber {
            number: from.number,
            hash: from.hash,
        });
        from = backend.header_metadata(from.parent)?;
    }

    // numbers are equal now. walk backwards until the block is the same

    while to.hash != from.hash {
        to_branch.push(HashAndNumber {
            number: to.number,
            hash: to.hash,
        });
        to = backend.header_metadata(to.parent)?;

        from_branch.push(HashAndNumber {
            number: from.number,
            hash: from.hash,
        });
        from = backend.header_metadata(from.parent)?;
    }

    // add the pivot block. and append the reversed to-branch (note that it's reverse order originals)
    let pivot = from_branch.len();
    from_branch.push(HashAndNumber {
        number: to.number,
        hash: to.hash,
    });
    from_branch.extend(to_branch.into_iter().rev());

    Ok(TreeRoute {
        route: from_branch,
        pivot,
    })
}

pub struct Client<S> {
    storage: Arc<S>,
}

impl<S> Client<S> {
    pub fn new(storage: Arc<S>) -> Self {
        Self {
            storage: storage.clone(),
        }
    }
}

impl<S> Clone for Client<S> {
    fn clone(&self) -> Self {
        Self {
            storage: self.storage.clone(),
        }
    }
}

impl<S, Block> HeaderMetadata<Block> for Client<S>
where
    Block: BlockT,
    S: Storage<Block>,
{
    /// Error used in case the header metadata is not found.
    type Error = BlockchainError;

    fn header_metadata(
        &self,
        hash: Block::Hash,
    ) -> Result<CachedHeaderMetadata<Block>, Self::Error> {
        self.storage.header_metadata(hash)
    }
}

impl<S, Block> HeaderBackend<Block> for Client<S>
where
    Block: BlockT,
    S: Storage<Block>,
{
    /// Get block header. Returns `None` if block is not found.
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        self.storage.header(id)
    }

    /// Get blockchain info.
    fn info(&self) -> BlockchainInfo<Block> {
        self.storage.info()
    }

    /// Get block status.
    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        self.storage.status(id)
    }

    /// Get block number by hash. Returns `None` if the header is not in the chain.
    fn number(
        &self,
        hash: Block::Hash,
    ) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
        self.storage.number(hash)
    }

    /// Get block hash by number. Returns `None` if the header is not in the chain.
    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        self.storage.hash(number)
    }
}

impl<S, Block> HeaderBackend<Block> for &Client<S>
where
    Block: BlockT,
    S: Storage<Block>,
{
    /// Get block header. Returns `None` if block is not found.
    fn header(&self, id: BlockId<Block>) -> BlockchainResult<Option<Block::Header>> {
        (**self).header(id)
    }

    /// Get blockchain info.
    fn info(&self) -> BlockchainInfo<Block> {
        (**self).info()
    }

    /// Get block status.
    fn status(&self, id: BlockId<Block>) -> BlockchainResult<BlockStatus> {
        (**self).status(id)
    }

    /// Get block number by hash. Returns `None` if the header is not in the chain.
    fn number(
        &self,
        hash: Block::Hash,
    ) -> BlockchainResult<Option<<<Block as BlockT>::Header as HeaderT>::Number>> {
        (**self).number(hash)
    }

    /// Get block hash by number. Returns `None` if the header is not in the chain.
    fn hash(&self, number: NumberFor<Block>) -> BlockchainResult<Option<Block::Hash>> {
        (**self).hash(number)
    }
}

impl<S, Block> BlockImport<Block> for &Client<S>
where
    Block: BlockT,
    S: Storage<Block>,
{
    type Error = ConsensusError;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        let BlockCheckParams {
            hash,
            number,
            parent_hash,
            allow_missing_state,
            import_existing,
        } = block;

        let block_status = |id: &BlockId<Block>| -> BlockchainResult<ImportBlockStatus> {
            let hash_and_number = match id.clone() {
                BlockId::Hash(hash) => self.number(hash)?.map(|n| (hash, n)),
                BlockId::Number(n) => self.hash(number)?.map(|hash| (hash, n)),
            };
            match hash_and_number {
                Some(_) => Ok(ImportBlockStatus::InChainWithState),
                None => Ok(ImportBlockStatus::Unknown),
            }
        };

        match block_status(&BlockId::Hash(hash))
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?
        {
            ImportBlockStatus::InChainWithState | ImportBlockStatus::Queued if !import_existing => {
                return Ok(ImportResult::AlreadyInChain)
            }
            ImportBlockStatus::InChainWithState
            | ImportBlockStatus::Queued
            | ImportBlockStatus::Unknown => {}
            ImportBlockStatus::InChainPruned => return Ok(ImportResult::AlreadyInChain),
            ImportBlockStatus::KnownBad => return Ok(ImportResult::KnownBad),
        }

        match block_status(&BlockId::Hash(parent_hash))
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?
        {
            ImportBlockStatus::InChainWithState | ImportBlockStatus::Queued => {}
            ImportBlockStatus::Unknown => return Ok(ImportResult::UnknownParent),
            ImportBlockStatus::InChainPruned if allow_missing_state => {}
            ImportBlockStatus::InChainPruned => return Ok(ImportResult::MissingState),
            ImportBlockStatus::KnownBad => return Ok(ImportResult::KnownBad),
        }
        Ok(ImportResult::imported(false))
    }

    fn import_block(
        &mut self,
        block: BlockImportParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        let BlockImportParams {
            origin: _,
            header,
            justification: _,
            auxiliary: _,
            fork_choice: _,
            intermediates,
            import_existing: _,
            ..
        } = block;

        if !intermediates.is_empty() {
            return Err(BlockchainError::IncompletePipeline)
                .map_err(|e| ConsensusError::ClientImport(e.to_string()).into());
        }

        let hash = header.hash();
        let status = self
            .storage
            .status(BlockId::Hash(hash))
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;

        match status {
            BlockStatus::InChain => return Ok(ImportResult::AlreadyInChain),
            BlockStatus::Unknown => {}
        }

        self.storage
            .import_header(header, NewBlockState::Best)
            .map_err(|e| ConsensusError::ClientImport(e.to_string()))?;

        Ok(ImportResult::imported(true))
    }
}

impl<S, Block> BlockImport<Block> for Client<S>
where
    Block: BlockT,
    S: Storage<Block>,
{
    type Error = ConsensusError;

    fn check_block(&mut self, block: BlockCheckParams<Block>) -> Result<ImportResult, Self::Error> {
        (&*self).check_block(block)
    }

    fn import_block(
        &mut self,
        block: BlockImportParams<Block>,
    ) -> Result<ImportResult, Self::Error> {
        (&*self).import_block(block)
    }
}

impl<S, Block> Finalizer<Block> for Client<S>
where
    Block: BlockT,
    S: Storage<Block>,
{
    fn finalize_block(
        &self,
        id: BlockId<Block>,
        _justification: Option<Vec<u8>>,
    ) -> BlockchainResult<()> {
        let possible_to_be_finalized_block = self.storage.header(id)?;
        if possible_to_be_finalized_block.is_none() {
            return Err(BlockchainError::UnknownBlock(format!(
                "Block: {:?} to be finalized not found in storage",
                id
            )));
        }
        let to_be_finalized = possible_to_be_finalized_block.unwrap().hash();

        let info = self.storage.info();
        let last_finalized = info.finalized_hash;
        let first_set_of_blocks_to_be_finalized = last_finalized == Default::default();

        let tree_route_from = if first_set_of_blocks_to_be_finalized {
            info.genesis_hash
        } else {
            last_finalized
        };

        if !first_set_of_blocks_to_be_finalized && to_be_finalized == last_finalized {
            return Ok(());
        }

        let route_to_be_finalized =
            tree_route(self.storage.as_ref(), tree_route_from, to_be_finalized)?;

        // Since we do not allow forks, retracted always needs to be empty and
        // enacted always need to be non-empty
        assert!(route_to_be_finalized.retracted().is_empty());
        assert!(!route_to_be_finalized.enacted().is_empty());

        let enacted = route_to_be_finalized.enacted();
        assert!(enacted.len() > 0);

        if first_set_of_blocks_to_be_finalized {
            self.storage
                .finalize_header(BlockId::Hash(tree_route_from))?;
        }

        for finalize_new in &enacted[..enacted.len() - 1] {
            self.storage
                .finalize_header(BlockId::Hash(finalize_new.hash))?;
        }

        assert_eq!(enacted.last().map(|e| e.hash), Some(to_be_finalized));

        self.storage
            .finalize_header(BlockId::Hash(to_be_finalized))?;

        Ok(())
    }
}

impl<S, Block> Finalizer<Block> for &Client<S>
where
    Block: BlockT,
    S: Storage<Block>,
{
    fn finalize_block(
        &self,
        id: BlockId<Block>,
        justification: Option<Vec<u8>>,
    ) -> BlockchainResult<()> {
        (**self).finalize_block(id, justification)
    }
}
