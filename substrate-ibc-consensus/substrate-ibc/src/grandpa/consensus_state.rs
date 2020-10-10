use crate::client::ClientType;
use codec::{Decode, Encode};
use sp_finality_grandpa::{AuthorityList, SetId};
use sp_runtime::RuntimeDebug;

// TODO: This struct is defined by grandpa client and it should be replaced with a serialized bytes.
// Also the ConsensusState MUST define a getTimestamp() method which returns the timestamp associated with that consensus state.

/// # Parameters
/// - `set_id`: This parameter will be encoded into payload with other data byt the function "localized_payload_with_buffer<E: Encode>". Note that according to the comments of method (https://crates.parity.io/sc_finality_grandpa/trait.GrandpaApi.html#method.generate_key_ownership_proof), current implementations ignore this parameter.
/// - `authorities`: A list of Grandpa authorities with associated weights.
/// - `commitment_root`: State root of a substrate block.
#[derive(Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct ConsensusState {
    pub root: crate::state::CommitmentRoot,
    pub height: u32,
    pub set_id: SetId,
    pub authorities: AuthorityList,
}

impl ConsensusState {
    pub fn new(
        root: crate::state::CommitmentRoot,
        height: u32,
        set_id: SetId,
        authorities: AuthorityList,
    ) -> Self {
        Self {
            root,
            height,
            set_id,
            authorities,
        }
    }
}

impl crate::state::ConsensusState for ConsensusState {
    fn client_type(&self) -> ClientType {
        ClientType::GRANDPA
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn root(&self) -> crate::state::CommitmentRoot {
        self.root
    }

    fn validate_basic(&self) {
        unimplemented!()
    }
}
