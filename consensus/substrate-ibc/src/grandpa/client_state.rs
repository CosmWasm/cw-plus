use crate::client::ClientType;
use codec::{Decode, Encode};
use sp_core::H256;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(Clone, Default, Encode, Decode, RuntimeDebug)]
pub struct ClientState {
    pub chain_id: H256,
    pub latest_height: u32,
    /// Block height when the client was frozen due to a misbehaviour by validator, e.g: Grandpa validaor
    pub frozen_height: Option<u32>,
    /// Connections opend by the client
    pub connections: Vec<H256>, // TODO: fixme! O(n)
    /// Connections opend by the client
    pub channels: Vec<(Vec<u8>, H256)>,
}

impl ClientState {
    pub fn new(chain_id: H256, latest_height: u32) -> Self {
        Self {
            chain_id: chain_id,
            latest_height: latest_height,
            frozen_height: None,
            connections: vec![],
            channels: vec![],
        }
    }
}

impl crate::state::ClientState for ClientState {
    fn chain_id(&self) -> H256 {
        self.chain_id
    }

    fn client_type(&self) -> ClientType {
        ClientType::GRANDPA
    }

    fn latest_height(&self) -> u32 {
        self.latest_height
    }

    fn is_frozen(&self) -> bool {
        self.frozen_height.is_some()
    }
}
