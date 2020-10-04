use crate::client::ClientType;
use codec::{Decode, Encode};
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::vec::Vec;
use sp_trie::StorageProof;

#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct Header {
    pub height: u32,
    pub block_hash: H256,
    pub commitment_root: H256,
    pub justification: Vec<u8>,
    pub authorities_proof: StorageProof,
}

impl crate::header::Header for Header {
    fn client_type(&self) -> ClientType {
        ClientType::GRANDPA
    }

    fn height(&self) -> u32 {
        self.height
    }
}
