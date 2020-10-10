use crate::common::types::consensus_error::ConsensusError;

/// Block import error.
#[derive(Debug)]
pub enum BlockImportError {
    /// Block missed header, can't be imported
    IncompleteHeader,
    /// Block verification failed, can't be imported
    VerificationFailed(String),
    /// Block is known to be Bad
    BadBlock,
    /// Parent state is missing.
    MissingState,
    /// Block has an unknown parent
    UnknownParent,
    /// Block import has been cancelled. This can happen if the parent block fails to be imported.
    Cancelled,
    /// Other error.
    Other(ConsensusError),
}
