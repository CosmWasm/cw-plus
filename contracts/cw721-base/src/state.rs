use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, StdResult, Storage};
use cw721::{ContractInfoResponse, Expiration};
use cw_storage_plus::{IndexedMap, Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    /// The owner of the newly minter NFT
    pub owner: CanonicalAddr,
    /// approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    pub approvals: Vec<Approval>,

    /// Identifies the asset to which this NFT represents
    pub name: String,
    /// Describes the asset to which this NFT represents
    pub description: String,
    /// A URI pointing to an image representing the asset
    pub image: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: CanonicalAddr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

pub const CONTRACT_INFO: Item<ContractInfoResponse> = Item::new(b"nft_info");
pub const MINTER: Item<CanonicalAddr> = Item::new(b"minter");
pub const TOKEN_COUNT: Item<u64> = Item::new(b"num_tokens");

// pub const TOKENS: Map<&str, TokenInfo> = Map::new(b"tokens");
pub const OPERATORS: Map<(&[u8], &[u8]), Expiration> = Map::new(b"operators");

pub fn num_tokens<S: Storage>(storage: &S) -> StdResult<u64> {
    Ok(TOKEN_COUNT.may_load(storage)?.unwrap_or_default())
}

pub fn increment_tokens<S: Storage>(storage: &mut S) -> StdResult<u64> {
    let val = num_tokens(storage)? + 1;
    TOKEN_COUNT.save(storage, &val)?;
    Ok(val)
}

pub const IDX_OWNER: &str = "owner";

// indexed map needs function, not const (for now at least)
pub fn tokens<'a, S: Storage + 'a>() -> IndexedMap<'a, 'a, &'a str, TokenInfo, S> {
    IndexedMap::<&str, TokenInfo, S>::new(b"tokens")
        .with_index(IDX_OWNER, b"tokens__owner", |d| d.owner.to_vec())
        .unwrap()
}
