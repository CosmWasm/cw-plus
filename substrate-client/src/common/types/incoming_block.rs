use sp_runtime::traits::Block as BlockT;
use sp_runtime::Justification;

/// Block data used by the queue.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IncomingBlock<B: BlockT> {
    /// Block header hash.
    pub hash: <B as BlockT>::Hash,
    /// Block header if requested.
    pub header: Option<<B as BlockT>::Header>,
    /// Block body if requested.
    pub body: Option<Vec<<B as BlockT>::Extrinsic>>,
    /// Justification if requested.
    pub justification: Option<Justification>,
    /// Allow importing the block skipping state verification if parent state is missing.
    pub allow_missing_state: bool,
    /// Re-validate existing block.
    pub import_existing: bool,
}
