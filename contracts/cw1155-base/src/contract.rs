use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, SubMsg,
    Uint128,
};
use cw_storage_plus::Bound;

use cw1155::{
    ApproveAllEvent, ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse,
    Cw1155BatchReceiveMsg, Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg, Expiration,
    IsApprovedForAllResponse, TokenId, TokenInfoResponse, TokensResponse, TransferEvent,
};
use cw2::set_contract_version;
use cw_utils::{maybe_addr, Event};

use crate::error::ContractError;
use crate::msg::InstantiateMsg;
use crate::state::{APPROVES, BALANCES, MINTER, TOKENS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1155-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

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
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
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
        } => execute_send_from(env, from, to, token_id, value, msg),
        Cw1155ExecuteMsg::BatchSendFrom {
            from,
            to,
            batch,
            msg,
        } => execute_batch_send_from(env, from, to, batch, msg),
        Cw1155ExecuteMsg::Mint {
            to,
            token_id,
            value,
            url,
            msg,
        } => execute_mint(env, to, token_id, value, url, msg),
        Cw1155ExecuteMsg::BatchMint { to, batch, msg } => execute_batch_mint(env, to, batch, msg),
        Cw1155ExecuteMsg::Burn {
            from,
            token_id,
            value,
        } => execute_burn(env, from, token_id, value),
        Cw1155ExecuteMsg::BatchBurn { from, batch } => execute_batch_burn(env, from, batch),
        Cw1155ExecuteMsg::ApproveAll { operator, expires } => {
            execute_approve_all(env, operator, expires)
        }
        Cw1155ExecuteMsg::RevokeAll { operator } => execute_revoke_all(env, operator),
    }
}

/// When from is None: mint new coins
/// When to is None: burn coins
/// When both are None: no token balance is changed, pointless but valid
///
/// Make sure permissions are checked before calling this.
fn execute_transfer_inner<'a>(
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

/// returns true iff the sender can execute approve or reject on the contract
fn check_can_approve(deps: Deps, env: &Env, owner: &Addr, operator: &Addr) -> StdResult<bool> {
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

fn guard_can_approve(
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

pub fn execute_send_from(
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

    let event = execute_transfer_inner(
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

pub fn execute_mint(
    env: ExecuteEnv,
    to: String,
    token_id: TokenId,
    amount: Uint128,
    url: Option<String>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;

    let to_addr = deps.api.addr_validate(&to)?;

    if info.sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let mut rsp = Response::default();

    let event = execute_transfer_inner(&mut deps, None, Some(&to_addr), &token_id, amount)?;
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
        let token_url = if let Some(url) = url {
            url
        } else {
            String::new()
        };
        TOKENS.save(deps.storage, &token_id, &token_url)?;
    }

    Ok(rsp)
}

pub fn execute_burn(
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
    let event = execute_transfer_inner(&mut deps, Some(&from_addr), None, &token_id, amount)?;
    event.add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn execute_batch_send_from(
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
        let event = execute_transfer_inner(
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

pub fn execute_batch_mint(
    env: ExecuteEnv,
    to: String,
    batch: Vec<(TokenId, Uint128, Option<String>)>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;
    if info.sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let to_addr = deps.api.addr_validate(&to)?;

    let mut rsp = Response::default();

    for (token_id, amount, url) in batch.iter() {
        let event = execute_transfer_inner(&mut deps, None, Some(&to_addr), token_id, *amount)?;
        event.add_attributes(&mut rsp);
        // insert if not exist
        if !TOKENS.has(deps.storage, token_id) {
            let token_url = if let Some(url) = url.clone() {
                url
            } else {
                String::new()
            };
            TOKENS.save(deps.storage, token_id, &token_url)?;
        }
    }

    if let Some(msg) = msg {
        rsp.messages = vec![SubMsg::new(
            Cw1155BatchReceiveMsg {
                operator: info.sender.to_string(),
                from: None,
                batch: batch
                    .iter()
                    .map(|(token_id, amount, _)| (token_id.clone(), *amount))
                    .collect(),
                msg,
            }
            .into_cosmos_msg(to)?,
        )]
    };

    Ok(rsp)
}

pub fn execute_batch_burn(
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
        let event = execute_transfer_inner(&mut deps, Some(&from_addr), None, &token_id, amount)?;
        event.add_attributes(&mut rsp);
    }
    Ok(rsp)
}

pub fn execute_approve_all(
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

pub fn execute_revoke_all(env: ExecuteEnv, operator: String) -> Result<Response, ContractError> {
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: Cw1155QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw1155QueryMsg::Balance { owner, token_id } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let balance = BALANCES
                .may_load(deps.storage, (&owner_addr, &token_id))?
                .unwrap_or_default();
            to_binary(&BalanceResponse { balance })
        }
        Cw1155QueryMsg::BatchBalance { owner, token_ids } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let balances = token_ids
                .into_iter()
                .map(|token_id| -> StdResult<_> {
                    Ok(BALANCES
                        .may_load(deps.storage, (&owner_addr, &token_id))?
                        .unwrap_or_default())
                })
                .collect::<StdResult<_>>()?;
            to_binary(&BatchBalanceResponse { balances })
        }
        Cw1155QueryMsg::IsApprovedForAll { owner, operator } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let operator_addr = deps.api.addr_validate(&operator)?;
            let approved = check_can_approve(deps, &env, &owner_addr, &operator_addr)?;
            to_binary(&IsApprovedForAllResponse { approved })
        }
        Cw1155QueryMsg::ApprovedForAll {
            owner,
            include_expired,
            start_after,
            limit,
        } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let start_addr = maybe_addr(deps.api, start_after)?;
            to_binary(&query_all_approvals(
                deps,
                env,
                owner_addr,
                include_expired.unwrap_or(false),
                start_addr,
                limit,
            )?)
        }
        Cw1155QueryMsg::TokenInfo { token_id } => {
            let url = TOKENS.load(deps.storage, &token_id)?;
            to_binary(&TokenInfoResponse { url })
        }
        Cw1155QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            to_binary(&query_tokens(deps, owner_addr, start_after, limit)?)
        }
        Cw1155QueryMsg::AllTokens { start_after, limit } => {
            to_binary(&query_all_tokens(deps, start_after, limit)?)
        }
    }
}

fn build_approval(item: StdResult<(Addr, Expiration)>) -> StdResult<cw1155::Approval> {
    item.map(|(addr, expires)| cw1155::Approval {
        spender: addr.into(),
        expires,
    })
}

fn query_all_approvals(
    deps: Deps,
    env: Env,
    owner: Addr,
    include_expired: bool,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<ApprovedForAllResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_ref().map(Bound::exclusive);

    let operators = APPROVES
        .prefix(&owner)
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(build_approval)
        .collect::<StdResult<_>>()?;
    Ok(ApprovedForAllResponse { operators })
}

fn query_tokens(
    deps: Deps,
    owner: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_ref().map(|s| Bound::exclusive(s.as_str()));

    let tokens = BALANCES
        .prefix(&owner)
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<_>>()?;
    Ok(TokensResponse { tokens })
}

fn query_all_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.as_ref().map(|s| Bound::exclusive(s.as_str()));
    let tokens = TOKENS
        .keys(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .collect::<StdResult<_>>()?;
    Ok(TokensResponse { tokens })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{OverflowError, StdError};

    use super::*;

    #[test]
    fn check_transfers() {
        // A long test case that try to cover as many cases as possible.
        // Summary of what it does:
        // - try mint without permission, fail
        // - mint with permission, success
        // - query balance of receipant, success
        // - try transfer without approval, fail
        // - approve
        // - transfer again, success
        // - query balance of transfer participants
        // - batch mint token2 and token3, success
        // - try batch transfer without approval, fail
        // - approve and try batch transfer again, success
        // - batch query balances
        // - user1 revoke approval to minter
        // - query approval status
        // - minter try to transfer, fail
        // - user1 burn token1
        // - user1 batch burn token2 and token3
        let token1 = "token1".to_owned();
        let token2 = "token2".to_owned();
        let token3 = "token3".to_owned();
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let user2 = String::from("user2");

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // invalid mint, user1 don't mint permission
        let mint_msg = Cw1155ExecuteMsg::Mint {
            to: user1.clone(),
            token_id: token1.clone(),
            value: 1u64.into(),
            url: None,
            msg: None,
        };
        assert!(matches!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.as_ref(), &[]),
                mint_msg.clone(),
            ),
            Err(ContractError::Unauthorized {})
        ));

        // valid mint
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                mint_msg,
            )
            .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("to", &user1)
        );

        // query balance
        assert_eq!(
            to_binary(&BalanceResponse {
                balance: 1u64.into()
            }),
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Balance {
                    owner: user1.clone(),
                    token_id: token1.clone(),
                }
            ),
        );

        let transfer_msg = Cw1155ExecuteMsg::SendFrom {
            from: user1.clone(),
            to: user2.clone(),
            token_id: token1.clone(),
            value: 1u64.into(),
            msg: None,
        };

        // not approved yet
        assert!(matches!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                transfer_msg.clone(),
            ),
            Err(ContractError::Unauthorized {})
        ));

        // approve
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(user1.as_ref(), &[]),
            Cw1155ExecuteMsg::ApproveAll {
                operator: minter.clone(),
                expires: None,
            },
        )
        .unwrap();

        // transfer
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                transfer_msg,
            )
            .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
                .add_attribute("to", &user2)
        );

        // query balance
        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Balance {
                    owner: user2.clone(),
                    token_id: token1.clone(),
                }
            ),
            to_binary(&BalanceResponse {
                balance: 1u64.into()
            }),
        );
        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Balance {
                    owner: user1.clone(),
                    token_id: token1.clone(),
                }
            ),
            to_binary(&BalanceResponse {
                balance: 0u64.into()
            }),
        );

        // batch mint token2 and token3
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                Cw1155ExecuteMsg::BatchMint {
                    to: user2.clone(),
                    batch: vec![
                        (token2.clone(), 1u64.into(), None),
                        (token3.clone(), 1u64.into(), None)
                    ],
                    msg: None
                },
            )
            .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token2)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("to", &user2)
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token3)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("to", &user2)
        );

        // invalid batch transfer, (user2 not approved yet)
        let batch_transfer_msg = Cw1155ExecuteMsg::BatchSendFrom {
            from: user2.clone(),
            to: user1.clone(),
            batch: vec![
                (token1.clone(), 1u64.into()),
                (token2.clone(), 1u64.into()),
                (token3.clone(), 1u64.into()),
            ],
            msg: None,
        };
        assert!(matches!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                batch_transfer_msg.clone(),
            ),
            Err(ContractError::Unauthorized {}),
        ));

        // user2 approve
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(user2.as_ref(), &[]),
            Cw1155ExecuteMsg::ApproveAll {
                operator: minter.clone(),
                expires: None,
            },
        )
        .unwrap();

        // valid batch transfer
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                batch_transfer_msg,
            )
            .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user2)
                .add_attribute("to", &user1)
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token2)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user2)
                .add_attribute("to", &user1)
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token3)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user2)
                .add_attribute("to", &user1)
        );

        // batch query
        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::BatchBalance {
                    owner: user1.clone(),
                    token_ids: vec![token1.clone(), token2.clone(), token3.clone()],
                }
            ),
            to_binary(&BatchBalanceResponse {
                balances: vec![1u64.into(), 1u64.into(), 1u64.into()]
            }),
        );

        // user1 revoke approval
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(user1.as_ref(), &[]),
            Cw1155ExecuteMsg::RevokeAll {
                operator: minter.clone(),
            },
        )
        .unwrap();

        // query approval status
        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::IsApprovedForAll {
                    owner: user1.clone(),
                    operator: minter.clone(),
                }
            ),
            to_binary(&IsApprovedForAllResponse { approved: false }),
        );

        // tranfer without approval
        assert!(matches!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                Cw1155ExecuteMsg::SendFrom {
                    from: user1.clone(),
                    to: user2,
                    token_id: token1.clone(),
                    value: 1u64.into(),
                    msg: None,
                },
            ),
            Err(ContractError::Unauthorized {})
        ));

        // burn token1
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.as_ref(), &[]),
                Cw1155ExecuteMsg::Burn {
                    from: user1.clone(),
                    token_id: token1.clone(),
                    value: 1u64.into(),
                }
            )
            .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
        );

        // burn them all
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.as_ref(), &[]),
                Cw1155ExecuteMsg::BatchBurn {
                    from: user1.clone(),
                    batch: vec![(token2.clone(), 1u64.into()), (token3.clone(), 1u64.into())]
                }
            )
            .unwrap(),
            Response::new()
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token2)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token3)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
        );
    }

    #[test]
    fn check_send_contract() {
        let receiver = String::from("receive_contract");
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let token1 = "token1".to_owned();
        let token2 = "token2".to_owned();
        let dummy_msg = Binary::default();

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token2.clone(),
                value: 1u64.into(),
                url: None,
                msg: None,
            },
        )
        .unwrap();

        // mint to contract
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.as_ref(), &[]),
                Cw1155ExecuteMsg::Mint {
                    to: receiver.clone(),
                    token_id: token1.clone(),
                    value: 1u64.into(),
                    url: None,
                    msg: Some(dummy_msg.clone()),
                },
            )
            .unwrap(),
            Response::new()
                .add_message(
                    Cw1155ReceiveMsg {
                        operator: minter.clone(),
                        from: None,
                        amount: 1u64.into(),
                        token_id: token1.clone(),
                        msg: dummy_msg.clone(),
                    }
                    .into_cosmos_msg(receiver.clone())
                    .unwrap()
                )
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token1)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("to", &receiver)
        );

        // BatchSendFrom
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.as_ref(), &[]),
                Cw1155ExecuteMsg::BatchSendFrom {
                    from: user1.clone(),
                    to: receiver.clone(),
                    batch: vec![(token2.clone(), 1u64.into())],
                    msg: Some(dummy_msg.clone()),
                },
            )
            .unwrap(),
            Response::new()
                .add_message(
                    Cw1155BatchReceiveMsg {
                        operator: user1.clone(),
                        from: Some(user1.clone()),
                        batch: vec![(token2.clone(), 1u64.into())],
                        msg: dummy_msg,
                    }
                    .into_cosmos_msg(receiver.clone())
                    .unwrap()
                )
                .add_attribute("action", "transfer")
                .add_attribute("token_id", &token2)
                .add_attribute("amount", 1u64.to_string())
                .add_attribute("from", &user1)
                .add_attribute("to", &receiver)
        );
    }

    #[test]
    fn check_queries() {
        // mint multiple types of tokens, and query them
        // grant approval to multiple operators, and query them
        let tokens = (0..10).map(|i| format!("token{}", i)).collect::<Vec<_>>();
        let users = (0..10).map(|i| format!("user{}", i)).collect::<Vec<_>>();
        let minter = String::from("minter");

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::BatchMint {
                to: users[0].clone(),
                batch: tokens
                    .iter()
                    .map(|token_id| (token_id.clone(), 1u64.into(), None))
                    .collect::<Vec<_>>(),
                msg: None,
            },
        )
        .unwrap();

        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Tokens {
                    owner: users[0].clone(),
                    start_after: None,
                    limit: Some(5),
                },
            ),
            to_binary(&TokensResponse {
                tokens: tokens[..5].to_owned()
            })
        );

        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Tokens {
                    owner: users[0].clone(),
                    start_after: Some("token5".to_owned()),
                    limit: Some(5),
                },
            ),
            to_binary(&TokensResponse {
                tokens: tokens[6..].to_owned()
            })
        );

        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::AllTokens {
                    start_after: Some("token5".to_owned()),
                    limit: Some(5),
                },
            ),
            to_binary(&TokensResponse {
                tokens: tokens[6..].to_owned()
            })
        );

        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo {
                    token_id: "token5".to_owned()
                },
            ),
            to_binary(&TokenInfoResponse { url: "".to_owned() })
        );

        for user in users[1..].iter() {
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(users[0].as_ref(), &[]),
                Cw1155ExecuteMsg::ApproveAll {
                    operator: user.clone(),
                    expires: None,
                },
            )
            .unwrap();
        }

        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::ApprovedForAll {
                    owner: users[0].clone(),
                    include_expired: None,
                    start_after: Some(String::from("user2")),
                    limit: Some(1),
                },
            ),
            to_binary(&ApprovedForAllResponse {
                operators: vec![cw1155::Approval {
                    spender: users[3].clone(),
                    expires: Expiration::Never {}
                }],
            })
        );
    }

    #[test]
    fn approval_expires() {
        let mut deps = mock_dependencies();
        let token1 = "token1".to_owned();
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let user2 = String::from("user2");

        let env = {
            let mut env = mock_env();
            env.block.height = 10;
            env
        };

        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token1,
                value: 1u64.into(),
                url: None,
                msg: None,
            },
        )
        .unwrap();

        // invalid expires should be rejected
        assert!(matches!(
            execute(
                deps.as_mut(),
                env.clone(),
                mock_info(user1.as_ref(), &[]),
                Cw1155ExecuteMsg::ApproveAll {
                    operator: user2.clone(),
                    expires: Some(Expiration::AtHeight(5)),
                },
            ),
            Err(_)
        ));

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(user1.as_ref(), &[]),
            Cw1155ExecuteMsg::ApproveAll {
                operator: user2.clone(),
                expires: Some(Expiration::AtHeight(100)),
            },
        )
        .unwrap();

        let query_msg = Cw1155QueryMsg::IsApprovedForAll {
            owner: user1,
            operator: user2,
        };
        assert_eq!(
            query(deps.as_ref(), env, query_msg.clone()),
            to_binary(&IsApprovedForAllResponse { approved: true })
        );

        let env = {
            let mut env = mock_env();
            env.block.height = 100;
            env
        };

        assert_eq!(
            query(deps.as_ref(), env, query_msg,),
            to_binary(&IsApprovedForAllResponse { approved: false })
        );
    }

    #[test]
    fn mint_overflow() {
        let mut deps = mock_dependencies();
        let token1 = "token1".to_owned();
        let minter = String::from("minter");
        let user1 = String::from("user1");

        let env = mock_env();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token1.clone(),
                value: u128::MAX.into(),
                url: None,
                msg: None,
            },
        )
        .unwrap();

        assert!(matches!(
            execute(
                deps.as_mut(),
                env,
                mock_info(minter.as_ref(), &[]),
                Cw1155ExecuteMsg::Mint {
                    to: user1,
                    token_id: token1,
                    value: 1u64.into(),
                    url: None,
                    msg: None,
                },
            ),
            Err(ContractError::Std(StdError::Overflow {
                source: OverflowError { .. },
                ..
            }))
        ));
    }

    #[test]
    fn token_url() {
        let minter = String::from("minter");
        let user1 = String::from("user1");
        let token1 = "token1".to_owned();
        let token2 = "token2".to_owned();
        let url1 = "url1".to_owned();
        let url2 = "url2".to_owned();

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // first mint
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token1.clone(),
                value: 1u64.into(),
                url: Some(url1.clone()),
                msg: None,
            },
        )
        .unwrap();

        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo {
                    token_id: token1.clone()
                }
            ),
            to_binary(&TokenInfoResponse { url: url1.clone() })
        );

        // mint after the first mint
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token1.clone(),
                value: 1u64.into(),
                url: Some(url2.clone()),
                msg: None,
            },
        )
        .unwrap();

        // url doesn't changed
        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo { token_id: token1 }
            ),
            to_binary(&TokenInfoResponse { url: url1.clone() })
        );

        // first mint with batch_mint
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::BatchMint {
                to: user1.clone(),
                batch: vec![(token2.clone(), 1u64.into(), Some(url1.clone()))],
                msg: None,
            },
        )
        .unwrap();

        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo {
                    token_id: token2.clone()
                }
            ),
            to_binary(&TokenInfoResponse { url: url1.clone() })
        );

        // mint after first mint with batch_mint
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.as_ref(), &[]),
            Cw1155ExecuteMsg::BatchMint {
                to: user1,
                batch: vec![(token2.clone(), 1u64.into(), Some(url2))],
                msg: None,
            },
        )
        .unwrap();

        // url doesn't changed
        assert_eq!(
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::TokenInfo { token_id: token2 }
            ),
            to_binary(&TokenInfoResponse { url: url1 })
        );
    }
}
