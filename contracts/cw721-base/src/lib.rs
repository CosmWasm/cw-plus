pub mod contract;
mod error;
pub mod msg;
pub mod state;

pub use crate::error::ContractError;

mod entry {
    use super::*;

    #[cfg(not(feature = "library"))]
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, StdResult};

    // This makes a conscious choice on the various generics used by the contract
    #[cfg_attr(not(feature = "library"), entry_point)]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: msg::InstantiateMsg,
    ) -> StdResult<Response> {
        let tract = contract::Cw721Contract::default();
        tract.instantiate(deps, env, info, msg)
    }
}
