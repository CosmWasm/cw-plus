use crate::msg::AdminListResponse;
use cosmwasm_std::{CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response};
use cw1::query::CanExecuteResponse;

pub trait Cw1Whitelist<T> {
    type Error;

    fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msgs: Vec<CosmosMsg<T>>,
    ) -> Result<Response<T>, Self::Error>;

    fn freeze(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
    ) -> Result<Response<T>, Self::Error>;

    fn update_admins(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        admins: Vec<String>,
    ) -> Result<Response<T>, Self::Error>;

    fn admin_list(&self, deps: Deps, env: Env) -> Result<AdminListResponse, Self::Error>;

    fn can_execute(
        &self,
        deps: Deps,
        env: Env,
        sender: String,
        msg: CosmosMsg<T>,
    ) -> Result<CanExecuteResponse, Self::Error>;
}
