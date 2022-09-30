/*!
This may be the simplest implementation of [CW1](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw1/README.md), a whitelist of addresses.
It contains a set of admins that are defined upon creation.
Any of those admins may `Execute` any message via the contract,
per the CW1 spec.

To make this slighly less minimalistic, you can allow the admin set
to be mutable or immutable. If it is mutable, then any admin may
(a) change the admin set and (b) freeze it (making it immutable).

While largely an example contract for CW1, this has various real-world use-cases,
such as a common account that is shared among multiple trusted devices,
or trading an entire account (used as 1 of 1 mutable). Most of the time,
this can be used as a framework to build your own,
more advanced cw1 implementations.

For more information on this contract, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/contracts/cw1-whitelist-ng/README.md).
*/

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
