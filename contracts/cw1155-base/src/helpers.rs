use cosmwasm_std::{Addr, Deps, Env, StdResult};

use crate::{state::APPROVES, ContractError};

/// returns true if the sender can execute approve or reject on the contract
pub fn check_can_approve(deps: Deps, env: &Env, owner: &Addr, operator: &Addr) -> StdResult<bool> {
    // owner can approve
    if owner == operator {
        return Ok(true);
    }
    // operator can approve
    let op = APPROVES.may_load(deps.storage, (owner, operator))?;
    Ok(match op {
        Some(ex) => !ex.is_expired(&env.block),
        None => false,
    })
}

pub fn guard_can_approve(
    deps: Deps,
    env: &Env,
    owner: &Addr,
    operator: &Addr,
) -> Result<(), ContractError> {
    if !check_can_approve(deps, env, owner, operator)? {
        Err(ContractError::Unauthorized {})
    } else {
        Ok(())
    }
}
