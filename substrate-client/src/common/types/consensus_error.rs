use core::fmt;
use std::error;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ConsensusError {
    /// Missing state at block with given descriptor.
    StateUnavailable(String),
    /// I/O terminated unexpectedly
    IoTerminated,
    /// Intermediate missing.
    NoIntermediate,
    /// Intermediate is of wrong type.
    InvalidIntermediate,
    /// Unable to schedule wake-up.
    FaultyTimer(std::io::Error),
    /// Invalid authorities set received from the runtime.
    InvalidAuthoritiesSet,
    /// Justification requirements not met.
    InvalidJustification,
    /// Some other error.
    Other(Box<dyn error::Error + Send>),
    /// Error from the client while importing
    ClientImport(String),
    /// Error from the client while importing
    ChainLookup(String),
}

impl Error for ConsensusError {}

impl Display for ConsensusError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateUnavailable(s) => write!(f, "State unavailable at block {}", s),
            Self::IoTerminated => write!(f, "I/O terminated unexpectedly."),
            Self::NoIntermediate => write!(f, "Missing intermediate."),
            Self::InvalidIntermediate => write!(f, "Invalid intermediate."),
            Self::FaultyTimer(e) => write!(f, "Timer error: {}", e),
            Self::InvalidAuthoritiesSet => {
                write!(f, "Current state of blockchain has invalid authorities set")
            }
            Self::InvalidJustification => write!(f, "Invalid justification."),
            Self::Other(e) => write!(f, "Other error: {}", e),
            Self::ClientImport(s) => write!(f, "Import failed: {}", s),
            Self::ChainLookup(s) => write!(f, "Chain lookup failed: {}", s),
        }
    }
}
