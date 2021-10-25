mod contract;
pub mod error;
pub mod interfaces;
pub mod msg;
pub mod multitest;
pub mod query;
pub mod state;

#[cfg(not(feature = "library"))]
mod entry_points {
    use crate::error::ContractError;
    use crate::state::Cw1WhitelistContract;
    use cosmwasm_std::{entry_point, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response};

    const CONTRACT: Cw1WhitelistContract<Empty> = Cw1WhitelistContract::native();

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        CONTRACT.entry_instantiate(deps, env, info, &msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: Binary,
    ) -> Result<Response, ContractError> {
        CONTRACT.entry_execute(deps, env, info, &msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: Binary) -> Result<Binary, ContractError> {
        CONTRACT.entry_query(deps, env, &msg)
    }
}

#[cfg(not(feature = "library"))]
pub use entry_points::*;
