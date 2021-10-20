use crate::msg::*;
use crate::state::Cw1WhitelistContract;
use anyhow::{bail, Result as AnyResult};
use cosmwasm_std::{from_slice, Binary, DepsMut, Env, MessageInfo, Reply, Response};
use cw_multi_test::Contract;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

impl<T> Contract<T> for Cw1WhitelistContract
where
    T: Clone + std::fmt::Debug + PartialEq + JsonSchema + DeserializeOwned,
{
    fn instantiate(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>> {
        let msg: InstantiateMsg = from_slice(&msg)?;
        let InstantiateMsg { admins, mutable } = msg;
        self.instantiate(deps, env, info, admins, mutable)
            .map_err(Into::into)
    }

    fn execute(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Vec<u8>,
    ) -> AnyResult<Response<T>> {
        let msg: ExecuteMsg<T> = from_slice(&msg)?;
        msg.dispatch(deps, env, info, self).map_err(Into::into)
    }

    fn query(&self, deps: cosmwasm_std::Deps, env: Env, msg: Vec<u8>) -> AnyResult<Binary> {
        let msg: QueryMsg<T> = from_slice(&msg)?;
        msg.dispatch(deps, env, self).map_err(Into::into)
    }

    fn sudo(&self, _deps: DepsMut, _env: Env, _msg: Vec<u8>) -> AnyResult<Response<T>> {
        bail!("sudo not implemented for contract")
    }

    fn reply(&self, _deps: DepsMut, _env: Env, _msg: Reply) -> AnyResult<Response<T>> {
        bail!("reply not implemented for contract")
    }

    fn migrate(&self, _deps: DepsMut, _env: Env, _msg: Vec<u8>) -> AnyResult<Response<T>> {
        bail!("migrate not implemented for contract")
    }
}
