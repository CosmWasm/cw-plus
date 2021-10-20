mod contract;
pub mod error;
pub mod interfaces;
pub mod msg;
pub mod query;
pub mod state;

#[cfg(not(feature = "library"))]
mod binary {
    use crate::error::ContractError;
    use crate::msg::*;
    use crate::state::Cw1WhitelistContract;
    use cosmwasm_std::{Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};

    const CONTRACT: Cw1WhitelistContract = Cw1WhitelistContract::new();

    use cosmwasm_std::entry_point;

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let InstantiateMsg { admins, mutable } = msg;
        CONTRACT.instantiate(deps, env, info, admins, mutable)
    }

    // This would be probably generated with `msg` being `Binary` or equivalent, and the
    // deserialization would be generated in. The idea is to allow deserialize different messages
    // (so different interfaces), trying top-to bottom and handle first successfully deserialized.
    //
    // There are two open questions:
    // 1. How to ensure the message doesn't deserialize to many message types? The simplest and
    //    probably best approach is not to. Just well define order in which messages are tried.
    // 2. Which error to return if no message type matches the received type. The easy approach is
    //    to return the first or the last failure, however I think the best would be somehow
    //    collect are failures and return accumulated error
    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg<Empty>,
    ) -> Result<Response, ContractError> {
        msg.dispatch(deps, env, info, &CONTRACT)
    }

    // Same note as for `execute`
    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        msg.dispatch(deps, env, &CONTRACT)
    }
}

#[cfg(not(feature = "library"))]
pub use binary::*;
