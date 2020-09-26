use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, HumanAddr, ReadonlyStorage, Storage};
use cosmwasm_storage::{
    bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket, ReadonlySingleton,
    Singleton,
};
use cw721::{ContractInfoResponse, Expiration};

pub const CONFIG_KEY: &[u8] = b"config";
pub const MINTER_KEY: &[u8] = b"minter";
pub const CONTRACT_INFO_KEY: &[u8] = b"nft_info";

pub const TOKEN_PREFIX: &[u8] = b"tokens";
pub const OPERATOR_PREFIX: &[u8] = b"operators";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TokenInfo {
    /// The owner of the newly minter NFT
    owner: CanonicalAddr,
    /// approvals are stored here, as we clear them all upon transfer and cannot accumulate much
    approvals: Vec<Approval>,

    /// Identifies the asset to which this NFT represents
    name: String,
    /// Describes the asset to which this NFT represents
    description: String,
    /// A URI pointing to an image representing the asset
    image: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Approval {
    /// Account that can transfer/send the token
    pub spender: CanonicalAddr,
    /// When the Approval expires (maybe Expiration::never)
    pub expires: Expiration,
}

pub fn contract_info<S: Storage>(storage: &mut S) -> Singleton<S, ContractInfoResponse> {
    singleton(storage, CONTRACT_INFO_KEY)
}

pub fn contract_info_read<S: ReadonlyStorage>(
    storage: &S,
) -> ReadonlySingleton<S, ContractInfoResponse> {
    singleton_read(storage, CONTRACT_INFO_KEY)
}

pub fn mint<S: Storage>(storage: &mut S) -> Singleton<S, HumanAddr> {
    singleton(storage, MINTER_KEY)
}

pub fn mint_read<S: ReadonlyStorage>(storage: &S) -> ReadonlySingleton<S, HumanAddr> {
    singleton_read(storage, MINTER_KEY)
}

pub fn tokens<S: Storage>(storage: &mut S) -> Bucket<S, TokenInfo> {
    bucket(TOKEN_PREFIX, storage)
}

pub fn tokens_read<S: ReadonlyStorage>(storage: &S) -> ReadonlyBucket<S, TokenInfo> {
    bucket_read(TOKEN_PREFIX, storage)
}

pub fn operators<'a, S: Storage>(
    storage: &'a mut S,
    owner: &CanonicalAddr,
) -> Bucket<'a, S, Expiration> {
    Bucket::multilevel(&[OPERATOR_PREFIX, owner.as_slice()], storage)
}

pub fn operators_read<'a, S: ReadonlyStorage>(
    storage: &'a S,
    owner: &CanonicalAddr,
) -> ReadonlyBucket<'a, S, Expiration> {
    ReadonlyBucket::multilevel(&[OPERATOR_PREFIX, owner.as_slice()], storage)
}
