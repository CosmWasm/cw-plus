use crate::common::types::consensus_error::ConsensusError;
use core::fmt;
use std::error;
use std::fmt::{Display, Formatter};

/// Substrate Client error
#[derive(Debug)]
pub enum BlockchainError {
    /// Consensus Error
    Consensus(ConsensusError),
    /// Backend error.
    Backend(String),
    /// Unknown block.
    UnknownBlock(String),
    /// Blockchain error.
    Blockchain(Box<BlockchainError>),
    /// Invalid authorities set received from the runtime.
    InvalidAuthoritiesSet,
    /// Error decoding header justification.
    JustificationDecode,
    /// Justification for header is correctly encoded, but invalid.
    BadJustification(String),
    /// Not available on light client.
    NotAvailableOnLightClient,
    /// Last finalized block not parent of current.
    NonSequentialFinalization(String),
    /// Last block imported not parent of current.
    NonSequentialImport(String),
    /// Safety violation: new best block not descendent of last finalized.
    NotInFinalizedChain,
    /// Incomplete block import pipeline.
    IncompletePipeline,
    /// A convenience variant for String
    Msg(String),
    /// Error while decoding data
    DataDecode(String),
}

impl error::Error for BlockchainError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            BlockchainError::Consensus(e) => Some(e),
            BlockchainError::Blockchain(e) => Some(e),
            _ => None,
        }
    }
}

impl<'a> From<&'a str> for BlockchainError {
    fn from(s: &'a str) -> Self {
        BlockchainError::Msg(s.into())
    }
}

impl BlockchainError {
    /// Chain a blockchain error.
    pub fn from_blockchain(e: Box<BlockchainError>) -> Self {
        BlockchainError::Blockchain(e)
    }
}

impl Display for BlockchainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            BlockchainError::Consensus(e) => write!(f, "Consensus: {}", e),
            BlockchainError::Backend(s) => write!(f, "Backend error: {}", s),
            BlockchainError::UnknownBlock(s) => write!(f, "UnknownBlock: {}", s),
            BlockchainError::Blockchain(chained_error) => {
                write!(f, "Blockchain: {}", chained_error)
            }
            BlockchainError::InvalidAuthoritiesSet => {
                write!(f, "Current state of blockchain has invalid authorities set")
            }
            BlockchainError::JustificationDecode => {
                write!(f, "error decoding justification for header")
            }
            BlockchainError::BadJustification(s) => {
                write!(f, "bad justification for header: {}", s)
            }
            BlockchainError::NotAvailableOnLightClient => write!(
                f,
                "This method is not currently available when running in light client mode"
            ),
            BlockchainError::NonSequentialFinalization(s) => write!(
                f,
                "Trying to finalize blocks in non-sequential order. {}",
                s
            ),
            BlockchainError::NonSequentialImport(s) => {
                write!(f, "Trying to import blocks in non-sequential order. {}", s)
            }
            BlockchainError::NotInFinalizedChain => write!(
                f,
                "Potential long-range attack: block not in finalized chain."
            ),
            BlockchainError::IncompletePipeline => write!(f, "Incomplete block import pipeline."),
            BlockchainError::Msg(s) => write!(f, "{}", s),
            BlockchainError::DataDecode(s) => write!(f, "Error while decoding data: {}", s),
        }
    }
}
