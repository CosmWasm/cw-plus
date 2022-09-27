use crate::{error::ContractError, execute, msg::InstantiateMsg, query, state::MINTER};
use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw1155::{Cw1155ExecuteMsg, Cw1155QueryMsg};
use cw2::set_contract_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1155-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let minter = deps.api.addr_validate(&msg.minter)?;
    MINTER.save(deps.storage, &minter)?;
    Ok(Response::default())
}

/// To mitigate clippy::too_many_arguments warning
pub struct ExecuteEnv<'a> {
    pub deps: DepsMut<'a>,
    pub env: Env,
    pub info: MessageInfo,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw1155ExecuteMsg,
) -> Result<Response, ContractError> {
    let env = ExecuteEnv { deps, env, info };
    match msg {
        Cw1155ExecuteMsg::SendFrom {
            from,
            to,
            token_id,
            value,
            msg,
        } => execute::send_from(env, from, to, token_id, value, msg),
        Cw1155ExecuteMsg::BatchSendFrom {
            from,
            to,
            batch,
            msg,
        } => execute::batch_send_from(env, from, to, batch, msg),
        Cw1155ExecuteMsg::Mint {
            to,
            token_id,
            value,
            msg,
        } => execute::mint(env, to, token_id, value, msg),
        Cw1155ExecuteMsg::BatchMint { to, batch, msg } => execute::batch_mint(env, to, batch, msg),
        Cw1155ExecuteMsg::Burn {
            from,
            token_id,
            value,
        } => execute::burn(env, from, token_id, value),
        Cw1155ExecuteMsg::BatchBurn { from, batch } => execute::batch_burn(env, from, batch),
        Cw1155ExecuteMsg::ApproveAll { operator, expires } => {
            execute::approve_all(env, operator, expires)
        }
        Cw1155ExecuteMsg::RevokeAll { operator } => execute::revoke_all(env, operator),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: Cw1155QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw1155QueryMsg::Balance { owner, token_id } => {
            to_binary(&query::balance(deps, owner, token_id)?)
        }
        Cw1155QueryMsg::BatchBalance { owner, token_ids } => {
            to_binary(&query::batch_balance(deps, owner, token_ids)?)
        }
        Cw1155QueryMsg::IsApprovedForAll { owner, operator } => {
            to_binary(&query::is_approved_for_all(deps, env, owner, operator)?)
        }
        Cw1155QueryMsg::ApprovedForAll {
            owner,
            include_expired,
            start_after,
            limit,
        } => to_binary(&query::approved_for_all(
            deps,
            env,
            owner,
            include_expired.unwrap_or(false),
            start_after,
            limit,
        )?),
        Cw1155QueryMsg::TokenInfo { token_id } => to_binary(&query::token_info(deps, token_id)?),
        Cw1155QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => to_binary(&query::tokens(deps, owner, start_after, limit)?),
        Cw1155QueryMsg::AllTokens { start_after, limit } => {
            to_binary(&query::all_tokens(deps, start_after, limit)?)
        }
    }
}
