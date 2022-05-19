pub(crate) mod contract;
pub mod error;
pub mod interfaces;
pub mod multitest;
pub mod query;
pub mod state;

pub mod msg {
    pub use crate::contract::msg::*;
    pub use crate::interfaces::{cw1_msg, whitelist, AdminListResponse};
}

#[cfg(not(feature = "library"))]
mod entry_points {
    use crate::error::ContractError;
    use crate::msg::{ExecMsg, InstantiateMsg, QueryMsg};
    use crate::state::Cw1WhitelistContract;
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};

    const CONTRACT: Cw1WhitelistContract<Empty> = Cw1WhitelistContract::native();

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        msg.dispatch(&CONTRACT, (deps, env, info))
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecMsg,
    ) -> Result<Response, ContractError> {
        msg.dispatch(&CONTRACT, (deps, env, info))
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
        msg.dispatch(&CONTRACT, (deps, env))
    }
}

#[cfg(not(feature = "library"))]
pub use entry_points::*;
