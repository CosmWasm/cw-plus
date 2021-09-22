use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{Binary, DepsMut, Empty, Env, MessageInfo, Response};
use cw0::Expiration;

// TODO: move this somewhere else... ideally cosmwasm-std
pub trait CustomMsg: Clone + std::fmt::Debug + PartialEq + JsonSchema {}

impl CustomMsg for Empty {}

pub trait Cw721<T, C>
where
    T: Serialize + DeserializeOwned + Clone,
    C: CustomMsg,
{
    type Err: ToString;

    fn transfer_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: String,
        token_id: String,
    ) -> Result<Response<C>, Self::Err>;

    fn send_nft(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract: String,
        token_id: String,
        msg: Binary,
    ) -> Result<Response<C>, Self::Err>;

    fn approve(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
        expires: Option<Expiration>,
    ) -> Result<Response<C>, Self::Err>;

    fn revoke(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        spender: String,
        token_id: String,
    ) -> Result<Response<C>, Self::Err>;

    fn approve_all(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        operator: String,
        expires: Option<Expiration>,
    ) -> Result<Response<C>, Self::Err>;

    fn revoke_all(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        operator: String,
    ) -> Result<Response<C>, Self::Err>;
}
