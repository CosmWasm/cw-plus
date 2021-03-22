use cosmwasm_std::{
    to_binary, Api, Binary, Deps, DepsMut, Env, HumanAddr, MessageInfo, Order, Response, StdResult,
    Uint128, KV,
};
use cw_storage_plus::Bound;

use cw0::{maybe_canonical, Event};
use cw1155::{
    ApproveAllEvent, ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse,
    Cw1155BatchReceiveMsg, Cw1155ExecuteMsg, Cw1155QueryMsg, Cw1155ReceiveMsg, Expiration,
    IsApprovedForAllResponse, TokenId, TokenInfoResponse, TokensResponse, TransferEvent,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::InitMsg;
use crate::state::{APPROVES, BALANCES, MINTER, TOKENS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1155-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InitMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let minter = deps.api.canonical_address(&msg.minter)?;
    MINTER.save(deps.storage, &minter)?;
    Ok(Response::default())
}

/// To mitigate clippy::too_many_arguments warning
pub struct ExecuteEnv<'a> {
    deps: DepsMut<'a>,
    env: Env,
    info: MessageInfo,
}

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
            msg,
        } => execute_mint(env, to, token_id, value, msg),
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
/// When both are None: not token balance is changed, meaningless but valid
fn execute_transfer_inner<'a>(
    deps: &'a mut DepsMut,
    from: Option<&'a HumanAddr>,
    to: Option<&'a HumanAddr>,
    token_id: &'a str,
    amount: Uint128,
) -> Result<TransferEvent<'a>, ContractError> {
    if let Some(from) = from {
        let from_raw = deps.api.canonical_address(from)?;
        BALANCES.update(
            deps.storage,
            (from_raw.as_slice(), token_id),
            |balance: Option<Uint128>| balance.unwrap_or_default() - amount,
        )?;
    }

    if let Some(to) = to {
        let canonical_to = deps.api.canonical_address(to)?;
        BALANCES.update(
            deps.storage,
            (canonical_to.as_slice(), token_id),
            |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
        )?;
    }

    Ok(TransferEvent {
        from,
        to,
        token_id,
        amount,
    })
}

/// returns true iff the sender can execute approve or reject on the contract
fn check_can_approve(
    deps: Deps,
    env: &Env,
    owner: &HumanAddr,
    operator: &HumanAddr,
) -> StdResult<bool> {
    // owner can approve
    let owner_raw = deps.api.canonical_address(owner)?;
    let operator_raw = deps.api.canonical_address(operator)?;
    if owner_raw == operator_raw {
        return Ok(true);
    }
    // operator can approve
    let op = APPROVES.may_load(deps.storage, (&owner_raw, &operator_raw))?;
    Ok(match op {
        Some(ex) => !ex.is_expired(&env.block),
        None => false,
    })
}

fn guard(cond: bool, err: ContractError) -> Result<(), ContractError> {
    if cond {
        Ok(())
    } else {
        Err(err)
    }
}

pub fn execute_send_from(
    env: ExecuteEnv,
    from: HumanAddr,
    to: HumanAddr,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv {
        mut deps,
        env,
        info,
    } = env;

    guard(
        check_can_approve(deps.as_ref(), &env, &from, &info.sender)?,
        ContractError::Unauthorized {},
    )?;

    let mut rsp = Response::default();

    let event = execute_transfer_inner(&mut deps, Some(&from), Some(&to), &token_id, amount)?;
    event.add_attributes(&mut rsp);

    rsp.messages = if let Some(msg) = msg {
        vec![Cw1155ReceiveMsg {
            operator: info.sender,
            from: Some(from.clone()),
            amount,
            token_id: token_id.clone(),
            msg,
        }
        .into_cosmos_msg(to)?]
    } else {
        vec![]
    };

    Ok(rsp)
}

pub fn execute_mint(
    env: ExecuteEnv,
    to: HumanAddr,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;

    let sender = deps.api.canonical_address(&info.sender)?;
    if sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let mut rsp = Response::default();

    let event = execute_transfer_inner(&mut deps, None, Some(&to), &token_id, amount)?;
    event.add_attributes(&mut rsp);

    rsp.messages = if let Some(msg) = msg {
        vec![Cw1155ReceiveMsg {
            operator: info.sender,
            from: None,
            amount,
            token_id: token_id.clone(),
            msg,
        }
        .into_cosmos_msg(to)?]
    } else {
        vec![]
    };

    // insert if not exist
    let key = TOKENS.key(&token_id);
    if deps.storage.get(&key).is_none() {
        key.save(deps.storage, &"".to_owned())?;
    }

    Ok(rsp)
}

pub fn execute_burn(
    env: ExecuteEnv,
    from: HumanAddr,
    token_id: TokenId,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let ExecuteEnv {
        mut deps,
        info,
        env,
    } = env;

    // whoever can transfer these tokens can burn
    guard(
        check_can_approve(deps.as_ref(), &env, &from, &info.sender)?,
        ContractError::Unauthorized {},
    )?;

    let mut rsp = Response::default();
    let event = execute_transfer_inner(&mut deps, Some(&from), None, &token_id, amount)?;
    event.add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn execute_batch_send_from(
    env: ExecuteEnv,
    from: HumanAddr,
    to: HumanAddr,
    batch: Vec<(TokenId, Uint128)>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv {
        mut deps,
        env,
        info,
    } = env;

    guard(
        check_can_approve(deps.as_ref(), &env, &from, &info.sender)?,
        ContractError::Unauthorized {},
    )?;

    let mut rsp = Response::default();
    for (token_id, amount) in batch.iter() {
        let event = execute_transfer_inner(&mut deps, Some(&from), Some(&to), token_id, *amount)?;
        event.add_attributes(&mut rsp);
    }

    rsp.messages = if let Some(msg) = msg {
        vec![Cw1155BatchReceiveMsg {
            operator: info.sender,
            from: Some(from),
            batch,
            msg,
        }
        .into_cosmos_msg(to)?]
    } else {
        vec![]
    };

    Ok(rsp)
}

pub fn execute_batch_mint(
    env: ExecuteEnv,
    to: HumanAddr,
    batch: Vec<(TokenId, Uint128)>,
    msg: Option<Binary>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { mut deps, info, .. } = env;
    let sender = deps.api.canonical_address(&info.sender)?;
    if sender != MINTER.load(deps.storage)? {
        return Err(ContractError::Unauthorized {});
    }

    let mut rsp = Response::default();

    for (token_id, amount) in batch.iter() {
        let event = execute_transfer_inner(&mut deps, None, Some(&to), &token_id, *amount)?;
        event.add_attributes(&mut rsp);
    }

    for (token_id, _) in batch.iter() {
        // insert if not exist
        let key = TOKENS.key(&token_id);
        if deps.storage.get(&key).is_none() {
            key.save(deps.storage, &"".to_owned())?;
        }
    }

    rsp.messages = if let Some(msg) = msg {
        vec![Cw1155BatchReceiveMsg {
            operator: info.sender,
            from: None,
            batch,
            msg,
        }
        .into_cosmos_msg(to)?]
    } else {
        vec![]
    };

    Ok(rsp)
}

pub fn execute_batch_burn(
    env: ExecuteEnv,
    from: HumanAddr,
    batch: Vec<(TokenId, Uint128)>,
) -> Result<Response, ContractError> {
    let ExecuteEnv {
        mut deps,
        info,
        env,
    } = env;

    guard(
        check_can_approve(deps.as_ref(), &env, &from, &info.sender)?,
        ContractError::Unauthorized {},
    )?;

    let mut rsp = Response::default();
    for (token_id, amount) in batch.into_iter() {
        let event = execute_transfer_inner(&mut deps, Some(&from), None, &token_id, amount)?;
        event.add_attributes(&mut rsp);
    }
    Ok(rsp)
}

pub fn execute_approve_all(
    env: ExecuteEnv,
    operator: HumanAddr,
    expires: Option<Expiration>,
) -> Result<Response, ContractError> {
    let ExecuteEnv { deps, info, env } = env;

    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    // set the operator for us
    let sender_raw = deps.api.canonical_address(&info.sender)?;
    let operator_raw = deps.api.canonical_address(&operator)?;
    APPROVES.save(deps.storage, (&sender_raw, &operator_raw), &expires)?;

    let mut rsp = Response::default();
    ApproveAllEvent {
        sender: &info.sender,
        operator: &operator,
        approved: true,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn execute_revoke_all(env: ExecuteEnv, operator: HumanAddr) -> Result<Response, ContractError> {
    let ExecuteEnv { deps, info, .. } = env;
    let sender_raw = deps.api.canonical_address(&info.sender)?;
    let operator_raw = deps.api.canonical_address(&operator)?;
    APPROVES.remove(deps.storage, (&sender_raw, &operator_raw));

    let mut rsp = Response::default();
    ApproveAllEvent {
        sender: &info.sender,
        operator: &operator,
        approved: false,
    }
    .add_attributes(&mut rsp);
    Ok(rsp)
}

pub fn query(deps: Deps, env: Env, msg: Cw1155QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw1155QueryMsg::Balance { owner, token_id } => {
            let canonical_owner = deps.api.canonical_address(&owner)?;
            let balance = BALANCES
                .may_load(deps.storage, (canonical_owner.as_slice(), &token_id))?
                .unwrap_or_default();
            to_binary(&BalanceResponse { balance })
        }
        Cw1155QueryMsg::BatchBalance { owner, token_ids } => {
            let canonical_owner = deps.api.canonical_address(&owner)?;
            let balances = token_ids
                .into_iter()
                .map(|token_id| -> StdResult<_> {
                    Ok(BALANCES
                        .may_load(deps.storage, (canonical_owner.as_slice(), &token_id))?
                        .unwrap_or_default())
                })
                .collect::<StdResult<_>>()?;
            to_binary(&BatchBalanceResponse { balances })
        }
        Cw1155QueryMsg::IsApprovedForAll { owner, operator } => {
            let approved = check_can_approve(deps, &env, &owner, &operator)?;
            to_binary(&IsApprovedForAllResponse { approved })
        }
        Cw1155QueryMsg::ApprovedForAll {
            owner,
            include_expired,
            start_after,
            limit,
        } => to_binary(&query_all_approvals(
            deps,
            env,
            owner,
            include_expired.unwrap_or(false),
            start_after,
            limit,
        )?),
        Cw1155QueryMsg::TokenInfo { token_id } => {
            let url = TOKENS.load(deps.storage, &token_id)?;
            to_binary(&TokenInfoResponse { url })
        }
        Cw1155QueryMsg::Tokens {
            owner,
            start_after,
            limit,
        } => to_binary(&query_tokens(deps, owner, start_after, limit)?),
        Cw1155QueryMsg::AllTokens { start_after, limit } => {
            to_binary(&query_all_tokens(deps, start_after, limit)?)
        }
    }
}

fn parse_approval(api: &dyn Api, item: StdResult<KV<Expiration>>) -> StdResult<cw1155::Approval> {
    item.and_then(|(k, expires)| {
        let spender = api.human_address(&k.into())?;
        Ok(cw1155::Approval { spender, expires })
    })
}

fn query_all_approvals(
    deps: Deps,
    env: Env,
    owner: HumanAddr,
    include_expired: bool,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<ApprovedForAllResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start_canon = maybe_canonical(deps.api, start_after)?;
    let start = start_canon.map(Bound::exclusive);

    let owner_raw = deps.api.canonical_address(&owner)?;
    let operators = APPROVES
        .prefix(&owner_raw)
        .range(deps.storage, start, None, Order::Ascending)
        .filter(|r| include_expired || r.is_err() || !r.as_ref().unwrap().1.is_expired(&env.block))
        .take(limit)
        .map(|item| parse_approval(deps.api, item))
        .collect::<StdResult<_>>()?;
    Ok(ApprovedForAllResponse { operators })
}

fn query_tokens(
    deps: Deps,
    owner: HumanAddr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let owner_raw = deps.api.canonical_address(&owner)?;
    let tokens = BALANCES
        .prefix(&owner_raw)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(k, _)| String::from_utf8(k).unwrap()))
        .collect::<StdResult<_>>()?;
    Ok(TokensResponse { tokens })
}

fn query_all_tokens(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);
    let tokens = TOKENS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(k, _)| String::from_utf8(k).unwrap()))
        .collect::<StdResult<_>>()?;
    Ok(TokensResponse { tokens })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::attr;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

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
        let minter: HumanAddr = "minter".into();
        let user1: HumanAddr = "user1".into();
        let user2: HumanAddr = "user2".into();

        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // invalid mint, user1 don't mint permission
        let mint_msg = Cw1155ExecuteMsg::Mint {
            to: user1.clone(),
            token_id: token1.clone(),
            value: 1u64.into(),
            msg: None,
        };
        assert!(matches!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.clone(), &[]),
                mint_msg.clone(),
            ),
            Err(ContractError::Unauthorized {})
        ));

        // valid mint
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.clone(), &[]),
                mint_msg,
            )
            .unwrap(),
            Response {
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token1),
                    attr("amount", 1u64),
                    attr("to", &user1),
                ],
                ..Response::default()
            }
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
                mock_info(minter.clone(), &[]),
                transfer_msg.clone(),
            ),
            Err(ContractError::Unauthorized {})
        ));

        // approve
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(user1.clone(), &[]),
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
                mock_info(minter.clone(), &[]),
                transfer_msg.clone(),
            )
            .unwrap(),
            Response {
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token1),
                    attr("amount", 1u64),
                    attr("from", &user1),
                    attr("to", &user2),
                ],
                ..Response::default()
            }
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
                mock_info(minter.clone(), &[]),
                Cw1155ExecuteMsg::BatchMint {
                    to: user2.clone(),
                    batch: vec![(token2.clone(), 1u64.into()), (token3.clone(), 1u64.into())],
                    msg: None
                },
            )
            .unwrap(),
            Response {
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token2),
                    attr("amount", 1u64),
                    attr("to", &user2),
                    attr("action", "transfer"),
                    attr("token_id", &token3),
                    attr("amount", 1u64),
                    attr("to", &user2),
                ],
                ..Response::default()
            }
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
                mock_info(minter.clone(), &[]),
                batch_transfer_msg.clone(),
            ),
            Err(ContractError::Unauthorized {}),
        ));

        // user2 approve
        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(user2.clone(), &[]),
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
                mock_info(minter.clone(), &[]),
                batch_transfer_msg,
            )
            .unwrap(),
            Response {
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token1),
                    attr("amount", 1u64),
                    attr("from", &user2),
                    attr("to", &user1),
                    attr("action", "transfer"),
                    attr("token_id", &token2),
                    attr("amount", 1u64),
                    attr("from", &user2),
                    attr("to", &user1),
                    attr("action", "transfer"),
                    attr("token_id", &token3),
                    attr("amount", 1u64),
                    attr("from", &user2),
                    attr("to", &user1),
                ],
                ..Response::default()
            },
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
            mock_info(user1.clone(), &[]),
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
                mock_info(minter.clone(), &[]),
                Cw1155ExecuteMsg::SendFrom {
                    from: user1.clone(),
                    to: user2.clone(),
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
                mock_info(user1.clone(), &[]),
                Cw1155ExecuteMsg::Burn {
                    from: user1.clone(),
                    token_id: token1.clone(),
                    value: 1u64.into(),
                }
            )
            .unwrap(),
            Response {
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token1),
                    attr("amount", 1u64),
                    attr("from", &user1),
                ],
                ..Response::default()
            }
        );

        // burn them all
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.clone(), &[]),
                Cw1155ExecuteMsg::BatchBurn {
                    from: user1.clone(),
                    batch: vec![(token2.clone(), 1u64.into()), (token3.clone(), 1u64.into())]
                }
            )
            .unwrap(),
            Response {
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token2),
                    attr("amount", 1u64),
                    attr("from", &user1),
                    attr("action", "transfer"),
                    attr("token_id", &token3),
                    attr("amount", 1u64),
                    attr("from", &user1),
                ],
                ..Response::default()
            }
        );
    }

    #[test]
    fn check_send_contract() {
        let receiver: HumanAddr = "receive_contract".into();
        let minter: HumanAddr = "minter".into();
        let user1: HumanAddr = "user1".into();
        let token1 = "token1".to_owned();
        let token2 = "token2".to_owned();
        let dummy_msg = Binary::default();

        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.clone(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token2.clone(),
                value: 1u64.into(),
                msg: None,
            },
        )
        .unwrap();

        // mint to contract
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(minter.clone(), &[]),
                Cw1155ExecuteMsg::Mint {
                    to: receiver.clone(),
                    token_id: token1.clone(),
                    value: 1u64.into(),
                    msg: Some(dummy_msg.clone()),
                },
            )
            .unwrap(),
            Response {
                messages: vec![Cw1155ReceiveMsg {
                    operator: minter.clone(),
                    from: None,
                    amount: 1u64.into(),
                    token_id: token1.clone(),
                    msg: dummy_msg.clone(),
                }
                .into_cosmos_msg(receiver.clone())
                .unwrap(),],
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token1),
                    attr("amount", 1u64),
                    attr("to", &receiver),
                ],
                ..Response::default()
            }
        );

        // BatchSendFrom
        assert_eq!(
            execute(
                deps.as_mut(),
                mock_env(),
                mock_info(user1.clone(), &[]),
                Cw1155ExecuteMsg::BatchSendFrom {
                    from: user1.clone(),
                    to: receiver.clone(),
                    batch: vec![(token2.clone(), 1u64.into())],
                    msg: Some(dummy_msg.clone()),
                },
            )
            .unwrap(),
            Response {
                messages: vec![Cw1155BatchReceiveMsg {
                    operator: user1.clone(),
                    from: Some(user1.clone()),
                    batch: vec![(token2.clone(), 1u64.into())],
                    msg: dummy_msg.clone(),
                }
                .into_cosmos_msg(receiver.clone())
                .unwrap()],
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token2),
                    attr("amount", 1u64),
                    attr("from", &user1),
                    attr("to", &receiver),
                ],
                ..Response::default()
            }
        );
    }

    #[test]
    fn check_queries() {
        // mint multiple types of tokens, and query them
        // grant approval to multiple operators, and query them
        let tokens = (0..10).map(|i| format!("token{}", i)).collect::<Vec<_>>();
        let users = (0..10)
            .map(|i| HumanAddr::from(format!("user{}", i)))
            .collect::<Vec<_>>();
        let minter: HumanAddr = "minter".into();

        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.clone(), &[]),
            Cw1155ExecuteMsg::BatchMint {
                to: users[0].clone(),
                batch: tokens
                    .iter()
                    .map(|token_id| (token_id.clone(), 1u64.into()))
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
                mock_info(users[0].clone(), &[]),
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
                    start_after: Some("user2".into()),
                    limit: Some(1),
                },
            ),
            to_binary(&ApprovedForAllResponse {
                operators: vec![cw1155::Approval {
                    // Not ordered in the same way as HumanAddr
                    spender: users[8].clone().into(),
                    expires: Expiration::Never {}
                }],
            })
        );
    }

    #[test]
    fn approval_expires() {
        let mut deps = mock_dependencies(&[]);
        let token1 = "token1".to_owned();
        let minter: HumanAddr = "minter".into();
        let user1: HumanAddr = "user1".into();
        let user2: HumanAddr = "user2".into();

        let env = {
            let mut env = mock_env();
            env.block.height = 10;
            env
        };

        let msg = InitMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(minter.clone(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token1.clone(),
                value: 1u64.into(),
                msg: None,
            },
        )
        .unwrap();

        // invalid expires should be rejected
        assert!(matches!(
            execute(
                deps.as_mut(),
                env.clone(),
                mock_info(user1.clone(), &[]),
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
            mock_info(user1.clone(), &[]),
            Cw1155ExecuteMsg::ApproveAll {
                operator: user2.clone(),
                expires: Some(Expiration::AtHeight(100)),
            },
        )
        .unwrap();

        let query_msg = Cw1155QueryMsg::IsApprovedForAll {
            owner: user1.clone(),
            operator: user2.clone(),
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
    #[should_panic(expected = "attempt to add with overflow")]
    fn mint_overflow() {
        let mut deps = mock_dependencies(&[]);
        let token1 = "token1".to_owned();
        let minter: HumanAddr = "minter".into();
        let user1: HumanAddr = "user1".into();

        let env = {
            let mut env = mock_env();
            env.block.height = 10;
            env
        };

        let msg = InitMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), env.clone(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(minter.clone(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token1.clone(),
                value: u128::MAX.into(),
                msg: None,
            },
        )
        .unwrap();

        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(minter.clone(), &[]),
            Cw1155ExecuteMsg::Mint {
                to: user1.clone(),
                token_id: token1.clone(),
                value: 1u64.into(),
                msg: None,
            },
        )
        .unwrap();
    }
}
