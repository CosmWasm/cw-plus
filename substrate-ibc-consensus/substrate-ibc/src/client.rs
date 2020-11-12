use crate::header::Header;
use crate::state::{ClientState, ConsensusState};

/// Type of IBC client.
pub enum ClientType {
    Loopback,
    SoloMachine,
    Tendermint,
    GRANDPA,
}

pub trait ClientDef {
    type Header: Header;
    type ClientState: ClientState;
    type ConsensusState: ConsensusState;

    fn check_header_and_update_state(
      &self,
     client_state: Self::ClientState,
     header: Self::Header,
     ) -> Result<(Self::ClientState, Self::ConsensusState), Box<dyn std::error::Error>>;

    fn verify_client_consensus_state(
     &self,
     client_state: &Self::ClientState,
     height: Height,
     prefix: &CommitmentPrefix,
     proof: &CommitmentProof,
     client_id: &ClientId,
     consensus_height: Height,
     expected_consensus_state: &AnyConsensusState,
     ) -> Result<(), Box<dyn std::error::Error>>;

    // /// Verify a `proof` that a connection state matches that of the input `connection_end`.
    // fn verify_connection_state(
    //     &self,
    //     client_state: &Self::ClientState,
    //     height: Height,
    //     prefix: &CommitmentPrefix,
    //     proof: &CommitmentProof,
    //     connection_id: &ConnectionId,
    //     expected_connection_end: &ConnectionEnd,
    // ) -> Result<(), Box<dyn std::error::Error>>;

    // /// Verify the client state for this chain that it is stored on the counterparty chain.
    // #[allow(clippy::too_many_arguments)]
    // fn verify_client_full_state(
    //     &self,
    //     _client_state: &Self::ClientState,
    //     height: Height,
    //     root: &CommitmentRoot,
    //     prefix: &CommitmentPrefix,
    //     client_id: &ClientId,
    //     proof: &CommitmentProof,
    //     client_state: &AnyClientState,
    // ) -> Result<(), Box<dyn std::error::Error>>;
}
