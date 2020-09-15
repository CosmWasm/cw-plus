use cosmwasm_std::{
    Api, Binary, Empty, Env, Extern, HandleResponse,
    InitResponse, Querier, StdResult, Storage,
};
use cw2::set_contract_version;

use crate::msg::{HandleMsg, InitMsg, QueryMsg};
use crate::state::{Voters, voters};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw3-fixed-multisig";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let total_weight = msg.voters.iter().map(|v| v.weight).sum();
    let cfg =  Voters{
        // TODO: map to canonical addresses?
        voters: msg.voters,
        required_weight: msg.required_weight,
        total_weight,
        max_voting_period: msg.max_voting_period,
    };
    voters(&mut deps.storage).save(&cfg)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse<Empty>> {
    match msg {
        // TODO
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        // TODO
    }
}

#[cfg(test)]
mod tests {
    // TODO
}
