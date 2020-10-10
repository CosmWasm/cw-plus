use crate::common::types::block_check_params::BlockCheckParams;
use crate::common::types::block_import_params::BlockImportParams;
use crate::common::types::import_result::ImportResult;
use sp_runtime::traits::Block as BlockT;

/// Block import trait.
pub trait BlockImport<B: BlockT> {
    /// The error type.
    type Error: std::error::Error + Send + 'static;

    /// Check block preconditions.
    fn check_block(&mut self, block: BlockCheckParams<B>) -> Result<ImportResult, Self::Error>;

    /// Import a block.
    ///
    /// Cached data can be accessed through the blockchain cache.
    fn import_block(&mut self, block: BlockImportParams<B>) -> Result<ImportResult, Self::Error>;
}
