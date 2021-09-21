use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

use cosmwasm_std::{DepsMut, Empty, Env, MessageInfo, Response};

// TODO: move this somewhere else... ideally cosmwasm-std
pub trait CustomMsg: Clone + std::fmt::Debug + PartialEq + JsonSchema {}

impl CustomMsg for Empty {}

pub trait Cw721<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    type Err: ToString;

    fn transfer_nft<C: CustomMsg>(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        recipient: String,
        token_id: String,
    ) -> Result<Response<C>, Self::Err>;
}
