use cosmwasm_std::{
    attr, to_binary, Binary, Deps, DepsMut, Env, HumanAddr, MessageInfo, Response, StdResult,
    Uint128,
};

use cw1155::{
    ApprovedForAllResponse, BalanceResponse, BatchBalanceResponse, Cw1155HandleMsg, Cw1155QueryMsg,
    Cw1155ReceiveMsg, TokenId,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::InitMsg;
use crate::state::{APPROVES, MINTER, TOKENS};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1155-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

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

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw1155HandleMsg,
) -> Result<Response, ContractError> {
    match msg {
        Cw1155HandleMsg::TransferFrom {
            from,
            to,
            token_id,
            value,
        } => execute_transfer_from(deps, env, info, from, to, token_id, value, None),
        Cw1155HandleMsg::SendFrom {
            from,
            contract,
            token_id,
            value,
            msg,
        } => execute_transfer_from(
            deps,
            env,
            info,
            from,
            Some(contract),
            token_id,
            value,
            Some(msg),
        ),
        Cw1155HandleMsg::BatchTransferFrom { from, to, batch } => {
            execute_batch_transfer_from(deps, env, info, from, to, batch, None)
        }
        Cw1155HandleMsg::BatchSendFrom {
            from,
            contract,
            batch,
            msg,
        } => execute_batch_transfer_from(deps, env, info, from, contract, batch, Some(msg)),
        Cw1155HandleMsg::SetApprovalForAll { operator, approved } => {
            execute_set_approval(deps, env, info, operator, approved)
        }
    }
}

fn execute_transfer_from_inner(
    deps: &mut DepsMut,
    _env: &Env,
    info: &MessageInfo,
    from: Option<HumanAddr>,
    to: Option<HumanAddr>,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Option<Binary>>,
) -> Result<Response, ContractError> {
    if let Some(from) = &from {
        let canonical_from = deps.api.canonical_address(from)?;
        TOKENS.update(
            deps.storage,
            (&token_id, canonical_from.as_slice()),
            |balance: Option<Uint128>| balance.unwrap_or_default() - amount,
        )?;
    }

    if let Some(to) = &to {
        let canonical_to = deps.api.canonical_address(to)?;
        TOKENS.update(
            deps.storage,
            (&token_id, canonical_to.as_slice()),
            |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
        )?;
    }

    let messages = match msg {
        Some(msg) => vec![Cw1155ReceiveMsg {
            operator: info.sender.clone(),
            from: from.clone(),
            amount,
            token_id: token_id.clone(),
            msg,
        }
        .into_cosmos_msg(to.clone().unwrap())?], // `to` in Send messages must not be None
        None => vec![],
    };

    let mut attributes = vec![
        attr("action", "transfer"),
        attr("token_id", token_id),
        attr("amount", amount),
    ];
    if let Some(from) = from {
        attributes.push(attr("from", from));
    }
    if let Some(to) = to {
        attributes.push(attr("to", to));
    }

    Ok(Response {
        attributes,
        messages,
        ..Response::default()
    })
}

pub fn execute_transfer_from(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    from: Option<HumanAddr>,
    to: Option<HumanAddr>,
    token_id: TokenId,
    amount: Uint128,
    msg: Option<Option<Binary>>,
) -> Result<Response, ContractError> {
    if let Some(from) = &from {
        let canonical_from = deps.api.canonical_address(&from)?;
        let canonical_sender = deps.api.canonical_address(&info.sender)?;
        let approved = APPROVES
            .may_load(deps.storage, canonical_from.as_slice())?
            .map_or(false, |op| op == canonical_sender);
        if !approved {
            return Err(ContractError::Unauthorized {});
        }
    } else {
        // check minting permission
        let canonical_sender = deps.api.canonical_address(&info.sender)?;
        if MINTER.load(deps.storage)? != canonical_sender {
            return Err(ContractError::Unauthorized {});
        }
    }
    if to.is_none() {
        // check burning permission
        let canonical_sender = deps.api.canonical_address(&info.sender)?;
        if MINTER.load(deps.storage)? != canonical_sender {
            return Err(ContractError::Unauthorized {});
        }
    }

    execute_transfer_from_inner(&mut deps, &env, &info, from, to, token_id, amount, msg)
}

pub fn execute_batch_transfer_from(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    from: HumanAddr,
    to: HumanAddr,
    batch: Vec<(TokenId, Uint128)>,
    msg: Option<Option<Binary>>,
) -> Result<Response, ContractError> {
    let canonical_from = deps.api.canonical_address(&from)?;
    let canonical_sender = deps.api.canonical_address(&info.sender)?;
    let approved = APPROVES
        .may_load(deps.storage, canonical_from.as_slice())?
        .map_or(false, |op| op == canonical_sender);
    if !approved {
        return Err(ContractError::Unauthorized {});
    }

    let mut rsp = Response::default();
    for (token_id, amount) in batch.into_iter() {
        let sub_rsp = execute_transfer_from_inner(
            &mut deps,
            &env,
            &info,
            Some(from.clone()),
            Some(to.clone()),
            token_id,
            amount,
            msg.clone(),
        )?;
        let Response {
            submessages,
            messages,
            attributes,
            ..
        } = sub_rsp;
        rsp.submessages.extend(submessages);
        rsp.messages.extend(messages);
        rsp.attributes.extend(attributes);
    }
    Ok(rsp)
}

pub fn execute_set_approval(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    operator: HumanAddr,
    approved: bool,
) -> Result<Response, ContractError> {
    let canonical_sender = deps.api.canonical_address(&info.sender)?;
    let canonical_operator = deps.api.canonical_address(&operator)?;
    if approved {
        APPROVES.save(
            deps.storage,
            canonical_sender.as_slice(),
            &canonical_operator,
        )?;
    } else {
        APPROVES.remove(deps.storage, canonical_sender.as_slice());
    }
    Ok(Response::default())
}

pub fn query(deps: Deps, _env: Env, msg: Cw1155QueryMsg) -> StdResult<Binary> {
    match msg {
        Cw1155QueryMsg::Balance { owner, token_id } => {
            let canonical_owner = deps.api.canonical_address(&owner)?;
            let balance = TOKENS
                .may_load(deps.storage, (&token_id, canonical_owner.as_slice()))?
                .unwrap_or_default();
            Ok(to_binary(&BalanceResponse { balance })?)
        }
        Cw1155QueryMsg::BatchBalance { owner, token_ids } => {
            let canonical_owner = deps.api.canonical_address(&owner)?;
            let balances = token_ids
                .into_iter()
                .map(|token_id| -> StdResult<_> {
                    Ok(TOKENS
                        .may_load(deps.storage, (&token_id, canonical_owner.as_slice()))?
                        .unwrap_or_default())
                })
                .collect::<StdResult<_>>()?;
            Ok(to_binary(&BatchBalanceResponse { balances })?)
        }
        Cw1155QueryMsg::ApprovedForAll { owner, spender } => {
            let canonical_owner = deps.api.canonical_address(&owner)?;
            let canonical_spender = deps.api.canonical_address(&spender)?;
            let approved = APPROVES
                .may_load(deps.storage, canonical_owner.as_slice())?
                .map_or(false, |op| op == canonical_spender);
            Ok(to_binary(&ApprovedForAllResponse { approved })?)
        }
    }
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    use super::*;

    #[test]
    fn mint_approve_and_transfer() {
        let token = "token1".to_owned();
        let minter: HumanAddr = "minter".into();
        let user1: HumanAddr = "user1".into();
        let user2: HumanAddr = "user2".into();

        let mut deps = mock_dependencies(&[]);
        let msg = InitMsg {
            minter: minter.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), mock_info("operator", &[]), msg).unwrap();
        assert_eq!(0, res.messages.len());

        // mint
        let rsp = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(minter.clone(), &[]),
            Cw1155HandleMsg::TransferFrom {
                from: None,
                to: Some(user1.clone()),
                token_id: token.clone(),
                value: 1u64.into(),
            },
        )
        .unwrap();
        assert_eq!(
            rsp,
            Response {
                attributes: vec![
                    attr("action", "transfer"),
                    attr("token_id", &token),
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
                    token_id: token.clone(),
                }
            ),
        );

        let transfer_msg = Cw1155HandleMsg::TransferFrom {
            from: Some(user1.clone()),
            to: Some(user2.clone()),
            token_id: token.clone(),
            value: 1u64.into(),
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
            Cw1155HandleMsg::SetApprovalForAll {
                operator: minter.clone(),
                approved: true,
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
                    attr("token_id", &token),
                    attr("amount", 1u64),
                    attr("from", &user1),
                    attr("to", &user2),
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
                    owner: user2.clone(),
                    token_id: token.clone(),
                }
            ),
        );
        assert_eq!(
            to_binary(&BalanceResponse {
                balance: 0u64.into()
            }),
            query(
                deps.as_ref(),
                mock_env(),
                Cw1155QueryMsg::Balance {
                    owner: user1.clone(),
                    token_id: token.clone(),
                }
            ),
        )
    }
}
