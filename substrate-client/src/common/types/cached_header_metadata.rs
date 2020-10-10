use sp_runtime::traits::{Block as BlockT, Header, NumberFor};

/// Cached header metadata. Used to efficiently traverse the tree.
#[derive(Debug, Clone)]
pub struct CachedHeaderMetadata<Block: BlockT> {
    /// Hash of the header.
    pub hash: Block::Hash,
    /// Block number.
    pub number: NumberFor<Block>,
    /// Hash of parent header.
    pub parent: Block::Hash,
    /// Hash of an ancestor header. Used to jump through the tree.
    ancestor: Block::Hash,
}

impl<Block: BlockT> From<&Block::Header> for CachedHeaderMetadata<Block> {
    fn from(header: &Block::Header) -> Self {
        CachedHeaderMetadata {
            hash: header.hash().clone(),
            number: header.number().clone(),
            parent: header.parent_hash().clone(),
            ancestor: header.parent_hash().clone(),
        }
    }
}
