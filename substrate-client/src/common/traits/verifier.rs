use crate::common::types::block_import_params::BlockImportParams;
use crate::common::types::block_origin::BlockOrigin;
use sp_runtime::traits::Block as BlockT;
use sp_runtime::Justification;

/// Verify a justification of a block
pub trait Verifier<B: BlockT>: Send + Sync {
    /// Verify the given data and return the BlockImportParams and an optional
    /// new set of validators to import. If not, err with an Error-Message
    /// presented to the User in the logs.
    fn verify(
        &mut self,
        origin: BlockOrigin,
        header: B::Header,
        justification: Option<Justification>,
        body: Option<Vec<B::Extrinsic>>,
    ) -> Result<BlockImportParams<B>, String>;
}
