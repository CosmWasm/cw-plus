use cosmwasm_std::{Addr, Binary, DepsMut, Response, StdResult, SubMsg, Uint128};
use cw1155::{ApproveAllEvent, Cw1155BatchReceiveMsg, Cw1155ReceiveMsg, TokenId, TransferEvent};
use cw_utils::{Event, Expiration};

use crate::{
    contract::ExecuteEnv,
    helpers::guard_can_approve,
    state::{APPROVES, BALANCES, MINTER, TOKENS},
    ContractError,
};

/// When from is None: mint new coins
/// When to is None: burn coins
/// When both are None: no token balance is changed, pointless but valid
///
/// Make sure permissions are checked before calling this.
fn transfer_inner<'a>(
    deps: &'a mut DepsMut,
    from: Option<&'a Addr>,
    to: Option<&'a Addr>,
    token_id: &'a str,
    amount: Uint128,
) -> Result<TransferEvent<'a>, ContractError> {
    if let Some(from_addr) = from {
        BALANCES.update(
            deps.storage,
            (from_addr, token_id),
            |balance: Option<Uint128>| -> StdResult<_> {
                Ok(balance.unwrap_or_default().checked_sub(amount)?)
            },
        )?;
    }

    if let Some(to_addr) = to {
        BALANCES.update(
            deps.storage,
            (to_addr, token_id),
            |balance: Option<Uint128>| -> StdResult<_> {
                Ok(balance.unwrap_or_default().checked_add(amount)?)
            },
        )?;
    }

    Ok(TransferEvent {
        from: from.map(|x| x.as_ref()),
        to: to.map(|x| x.as_ref()),
        token_id,
        amount,
    })
}

pub fn send_from(
    env: ExecuteEnv,
    from: String,
    to: String,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let from_addr = env.deps.api.addr_validate(&from)?;
    let to_addr = env.deps.api.addr_validate(&to)?;

    let ExecuteEnv {
        mut deps,
        env,
        info,
    } = env;

    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = Response::default();

    let event = transfer_inner(
        &mut deps,
        Some(&from_addr),
        Some(&to_addr),
        &token_id,
        amount,
    )?;
    event.add_attributes(&mut rsp);

    if let Some(msg) = msg {
        rsp.messages = vec![SubMsg::new(
            Cw1155ReceiveMsg {
                operator: info.sender.to_string(),
                from: Some(from),
                amount,
                token_id: token_id.clone(),
                msg,
            }
            .into_cosmos_msg(to)?,
        )]
    }

    Ok(rsp)
}

pub fn mint(
    env: ExecuteEnv,
    to: String,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;

    let to_addr = deps.api.addr_validate(&to)?;

    if info.sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let mut rsp = Response::default();

    let event = transfer_inner(&mut deps, None, Some(&to_addr), &token_id, amount)?;
    event.add_attributes(&mut rsp);

    if let Some(msg) = msg {
        rsp.messages = vec![SubMsg::new(
            Cw1155ReceiveMsg {
                operator: info.sender.to_string(),
                from: None,
                amount,
                token_id: token_id.clone(),
                msg,
            }
            .into_cosmos_msg(to)?,
        )]
    }

    // insert if not exist
    if !TOKENS.has(deps.storage, &token_id) {
        // we must save some valid data here
        TOKENS.save(deps.storage, &token_id, &String::new())?;
    }

    Ok(rsp)
}

pub fn burn(
    env: ExecuteEnv,
    from: String,
    token_id: TokenId,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let ExecuteEnv {
        mut deps,
        info,
        env,
    } = env;

    let from_addr = deps.api.addr_validate(&from)?;

    // whoever can transfer these tokens can burn
    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = Response::default();
    let event = transfer_inner(&mut deps, Some(&from_addr), None, &token_id, amount)?;
    event.add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn batch_send_from(
    env: ExecuteEnv,
    from: String,
    to: String,
    batch: Vec<(TokenId, Uint128)>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv {
        mut deps,
        env,
        info,
    } = env;

    let from_addr = deps.api.addr_validate(&from)?;
    let to_addr = deps.api.addr_validate(&to)?;

    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = Response::default();
    for (token_id, amount) in batch.iter() {
        let event = transfer_inner(
            &mut deps,
            Some(&from_addr),
            Some(&to_addr),
            token_id,
            *amount,
        )?;
        event.add_attributes(&mut rsp);
    }

    if let Some(msg) = msg {
        rsp.messages = vec![SubMsg::new(
            Cw1155BatchReceiveMsg {
                operator: info.sender.to_string(),
                from: Some(from),
                batch,
                msg,
            }
            .into_cosmos_msg(to)?,
        )]
    };

    Ok(rsp)
}

pub fn batch_mint(
    env: ExecuteEnv,
    to: String,
    batch: Vec<(TokenId, Uint128)>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;
    if info.sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let to_addr = deps.api.addr_validate(&to)?;

    let mut rsp = Response::default();

    for (token_id, amount) in batch.iter() {
        let event = transfer_inner(&mut deps, None, Some(&to_addr), token_id, *amount)?;
        event.add_attributes(&mut rsp);

        // insert if not exist
        if !TOKENS.has(deps.storage, token_id) {
            // we must save some valid data here
            TOKENS.save(deps.storage, token_id, &String::new())?;
        }
    }

    if let Some(msg) = msg {
        rsp.messages = vec![SubMsg::new(
            Cw1155BatchReceiveMsg {
                operator: info.sender.to_string(),
                from: None,
                batch,
                msg,
            }
            .into_cosmos_msg(to)?,
        )]
    };

    Ok(rsp)
}

pub fn batch_burn(
    env: ExecuteEnv,
    from: String,
    batch: Vec<(TokenId, Uint128)>,
) -> Result<Response, ContractError> {
    let ExecuteEnv {
        mut deps,
        info,
        env,
    } = env;

    let from_addr = deps.api.addr_validate(&from)?;

    guard_can_approve(deps.as_ref(), &env, &from_addr, &info.sender)?;

    let mut rsp = Response::default();
    for (token_id, amount) in batch.into_iter() {
        let event = transfer_inner(&mut deps, Some(&from_addr), None, &token_id, amount)?;
        event.add_attributes(&mut rsp);
    }
    Ok(rsp)
}

pub fn approve_all(
    env: ExecuteEnv,
    operator: String,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { deps, info, env } = env;

    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // set the operator for us
    let operator_addr = deps.api.addr_validate(&operator)?;
    APPROVES.save(deps.storage, (&info.sender, &operator_addr), &expires)?;

    let mut rsp = Response::default();
    ApproveAllEvent {
        sender: info.sender.as_ref(),
        operator: &operator,
        approved: true,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn revoke_all(env: ExecuteEnv, operator: String) -> Result<Response, ContractError> {
    let ExecuteEnv { deps, info, .. } = env;
    let operator_addr = deps.api.addr_validate(&operator)?;
    APPROVES.remove(deps.storage, (&info.sender, &operator_addr));

    let mut rsp = Response::default();
    ApproveAllEvent {
        sender: info.sender.as_ref(),
        operator: &operator,
        approved: false,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}
