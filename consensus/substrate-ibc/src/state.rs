use crate::client::ClientType;
use sp_core::H256;

pub type CommitmentRoot = H256;

pub trait ConsensusState {
    /// Type of client associated with this consensus state (eg. Tendermint)
    fn client_type(&self) -> ClientType;

    /// Height of consensus state
    fn height(&self) -> u32;

    /// Commitment root of the consensus state, which is used for key-value pair verification.
    fn root(&self) -> CommitmentRoot;

    /// Performs basic validation of the consensus state
    fn validate_basic(&self);
}

pub trait ClientState {
    /// Client ID of this state
    fn chain_id(&self) -> H256;

    /// Type of client associated with this state (eg. Tendermint)
    fn client_type(&self) -> ClientType;

    /// Latest height of consensus state
    fn latest_height(&self) -> u32;

    /// Freeze status of the client
    fn is_frozen(&self) -> bool;
}
