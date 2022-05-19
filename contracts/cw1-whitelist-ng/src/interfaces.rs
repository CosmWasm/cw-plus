use cosmwasm_std::{CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError};
use cw1::query::CanExecuteResponse;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AdminListResponse {
    pub admins: Vec<String>,
    pub mutable: bool,
}

#[cw_derive::interface(module=cw1_msg, msg_type=T)]
pub trait Cw1<T>
where
    T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
{
    type Error: From<StdError>;

    #[msg(exec)]
    fn execute(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        msgs: Vec<CosmosMsg<T>>,
    ) -> Result<Response<T>, Self::Error>;

    #[msg(query)]
    fn can_execute(
        &self,
        ctx: (Deps, Env),
        sender: String,
        msg: CosmosMsg<T>,
    ) -> Result<CanExecuteResponse, Self::Error>;
}

#[cw_derive::interface(module=whitelist, msg_type=T)]
pub trait Whitelist<T>
where
    T: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
{
    type Error: From<StdError>;

    #[msg(exec)]
    fn freeze(&self, ctx: (DepsMut, Env, MessageInfo)) -> Result<Response<T>, Self::Error>;

    #[msg(exec)]
    fn update_admins(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        admins: Vec<String>,
    ) -> Result<Response<T>, Self::Error>;

    #[msg(query)]
    fn admin_list(&self, ctx: (Deps, Env)) -> Result<AdminListResponse, Self::Error>;
}
