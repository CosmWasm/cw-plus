use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw721::ContractInfoResponse;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct RoyaltiesInfo {
    pub royalty_payments: bool,
    /// This is how much the minter takes as a cut when sold
    pub royalty_percentage: Option<u32>,
    /// The payment address, may be different to or the same
    /// as the minter addr
    pub royalty_payment_address: Option<Addr>,
}

// maps token id to royalties info
pub const ROYALTIES_INFO: Map<&str, RoyaltiesInfo> = Map::new("royalties_info");
pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new("nft_info");
