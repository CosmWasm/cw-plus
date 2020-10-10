use crate::common::types::block_origin::BlockOrigin;
use crate::common::types::consensus_error::ConsensusError;
use crate::common::types::fork_choice_strategy::ForkChoiceStrategy;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::Justification;
use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;

/// Data required to import a Block.
#[non_exhaustive]
pub struct BlockImportParams<Block: BlockT> {
    /// Origin of the Block
    pub origin: BlockOrigin,
    /// The header, without consensus post-digests applied. This should be in the same
    /// state as it comes out of the runtime.
    ///
    /// Consensus engines which alter the header (by adding post-runtime digests)
    /// should strip those off in the initial verification process and pass them
    /// via the `post_digests` field. During block authorship, they should
    /// not be pushed to the header directly.
    ///
    /// The reason for this distinction is so the header can be directly
    /// re-executed in a runtime that checks digest equivalence -- the
    /// post-runtime digests are pushed back on after.
    pub header: Block::Header,
    /// Justification provided for this block from the outside.
    pub justification: Option<Justification>,
    /// Intermediate values that are interpreted by block importers. Each block importer,
    /// upon handling a value, removes it from the intermediate list. The final block importer
    /// rejects block import if there are still intermediate values that remain unhandled.
    pub intermediates: HashMap<Cow<'static, [u8]>, Box<dyn Any>>,
    /// Auxiliary consensus data produced by the block.
    /// Contains a list of key-value pairs. If values are `None`, the keys
    /// will be deleted.
    pub auxiliary: Vec<(Vec<u8>, Option<Vec<u8>>)>,
    /// Fork choice strategy of this import. This should only be set by a
    /// synchronous import, otherwise it may race against other imports.
    /// `None` indicates that the current verifier or importer cannot yet
    /// determine the fork choice value, and it expects subsequent importer
    /// to modify it. If `None` is passed all the way down to bottom block
    /// importer, the import fails with an `IncompletePipeline` error.
    pub fork_choice: Option<ForkChoiceStrategy>,
    /// Allow importing the block skipping state verification if parent state is missing.
    pub allow_missing_state: bool,
    /// Re-validate existing block.
    pub import_existing: bool,
}

impl<Block> BlockImportParams<Block>
where
    Block: BlockT,
{
    pub fn new(origin: BlockOrigin, header: Block::Header) -> Self {
        Self {
            origin,
            header,
            justification: None,
            intermediates: HashMap::new(),
            auxiliary: vec![],
            fork_choice: None,
            allow_missing_state: false,
            import_existing: false,
        }
    }

    /// Take intermediate by given key, and remove it from the processing list.
    pub fn take_intermediate<T: 'static>(&mut self, key: &[u8]) -> Result<Box<T>, ConsensusError> {
        let (k, v) = self
            .intermediates
            .remove_entry(key)
            .ok_or(ConsensusError::NoIntermediate)?;

        match v.downcast::<T>() {
            Ok(v) => Ok(v),
            Err(v) => {
                self.intermediates.insert(k, v);
                Err(ConsensusError::InvalidIntermediate)
            }
        }
    }
}
