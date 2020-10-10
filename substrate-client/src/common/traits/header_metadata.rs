use crate::common::types::cached_header_metadata::CachedHeaderMetadata;
use sp_runtime::traits::Block as BlockT;

/// Handles header metadata: hash, number, parent hash, etc.
pub trait HeaderMetadata<Block: BlockT> {
    /// Error used in case the header metadata is not found.
    type Error;

    fn header_metadata(
        &self,
        hash: Block::Hash,
    ) -> Result<CachedHeaderMetadata<Block>, Self::Error>;
}
