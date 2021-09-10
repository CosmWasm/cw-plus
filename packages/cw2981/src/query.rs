use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// These are the queries you will need to define royalty options
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw2981QueryMsg {
    // Should be called on sale to see if royalties are owed
    // by the marketplace selling the NFT.
    // See https://eips.ethereum.org/EIPS/eip-2981
    RoyaltyInfo {
        token_id: String,
        // the denom of this sale must also be the denom returned by RoyaltiesInfoResponse
        // this was originally implemented as a Coin
        // however that would mean you couldn't buy using CW20s
        // as CW20 is just mapping of addr -> balance
        sale_price: u128,
    },
    // Called against contract to determine if this NFT contract
    // implements royalties
    CheckRoyalties {},
}

/// Shows royalty info
/// These are always the same, just the implementation changes
/// depending on the granularity of the above queries.
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct RoyaltiesInfoResponse {
    pub address: String,
    // Note that this must be the same denom as that passed in to RoyaltyInfo
    // rounding up or down is at the discretion of the implementer
    pub royalty_amount: u128,
}

/// Shows if the contract implements royalties
/// if royalty_payments is true, marketplaces should pay them
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct CheckRoyaltiesResponse {
    pub royalty_payments: bool,
}
