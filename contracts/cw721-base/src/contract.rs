use cosmwasm_std::{
    attr, from_binary, to_binary, Api, Binary, CosmosMsg, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Order, Querier, StdError, StdResult, Storage,
};

use cw0::{calc_range_start_human, calc_range_start_string};
use cw2::set_contract_version;
use cw721::{
    AllNftInfoResponse, ApprovedForAllResponse, ContractInfoResponse, Expiration, NftInfoResponse,
    NumTokensResponse, OwnerOfResponse, TokensResponse,
};

use crate::msg::{HandleMsg, InitMsg, MinterResponse, QueryMsg};
use crate::state::{
    contract_info, contract_info_read, increment_tokens, mint, mint_read, num_tokens, operators,
    operators_read, tokens, tokens_read, Approval, TokenInfo,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw721-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    set_contract_version(&mut deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let info = ContractInfoResponse {
        name: msg.name,
        symbol: msg.symbol,
    };
    contract_info(&mut deps.storage).save(&info)?;
    let minter = deps.api.canonical_address(&msg.minter)?;
    mint(&mut deps.storage).save(&minter)?;
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Mint {
            token_id,
            owner,
            name,
            description,
            image,
        } => handle_mint(deps, env, token_id, owner, name, description, image),
        HandleMsg::Approve {
            spender,
            token_id,
            expires,
        } => handle_approve(deps, env, spender, token_id, expires),
        HandleMsg::Revoke { spender, token_id } => handle_revoke(deps, env, spender, token_id),
        HandleMsg::ApproveAll { operator, expires } => {
            handle_approve_all(deps, env, operator, expires)
        }
        HandleMsg::RevokeAll { operator } => handle_revoke_all(deps, env, operator),
        HandleMsg::TransferNft {
            recipient,
            token_id,
        } => handle_transfer_nft(deps, env, recipient, token_id),
        HandleMsg::SendNft {
            contract,
            token_id,
            msg,
        } => handle_send_nft(deps, env, contract, token_id, msg),
    }
}

pub fn handle_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    token_id: String,
    owner: HumanAddr,
    name: String,
    description: Option<String>,
    image: Option<String>,
) -> StdResult<HandleResponse> {
    let minter = mint(&mut deps.storage).load()?;
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;

    if sender_raw != minter {
        return Err(StdError::unauthorized());
    }

    // create the token
    let token = TokenInfo {
        owner: deps.api.canonical_address(&owner)?,
        approvals: vec![],
        name,
        description: description.unwrap_or_default(),
        image,
    };
    tokens(&mut deps.storage).update(token_id.as_bytes(), |old| match old {
        Some(_) => Err(StdError::generic_err("token_id already claimed")),
        None => Ok(token),
    })?;

    increment_tokens(&mut deps.storage)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "mint"),
            attr("minter", env.message.sender),
            attr("token_id", token_id),
        ],
        data: None,
    })
}

pub fn handle_transfer_nft<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: HumanAddr,
    token_id: String,
) -> StdResult<HandleResponse> {
    _transfer_nft(deps, &env, &recipient, &token_id)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "transfer_nft"),
            attr("sender", env.message.sender),
            attr("recipient", recipient),
            attr("token_id", token_id),
        ],
        data: None,
    })
}

pub fn handle_send_nft<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    contract: HumanAddr,
    token_id: String,
    msg: Option<Binary>,
) -> StdResult<HandleResponse> {
    // Unwrap message first
    let msgs: Vec<CosmosMsg> = match &msg {
        None => vec![],
        Some(msg) => vec![from_binary(msg)?],
    };

    // Transfer token
    _transfer_nft(deps, &env, &contract, &token_id)?;

    // Send message
    Ok(HandleResponse {
        messages: msgs,
        attributes: vec![
            attr("action", "send_nft"),
            attr("sender", env.message.sender),
            attr("recipient", contract),
            attr("token_id", token_id),
        ],
        data: None,
    })
}

pub fn _transfer_nft<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    recipient: &HumanAddr,
    token_id: &str,
) -> StdResult<TokenInfo> {
    let mut token = tokens(&mut deps.storage).load(token_id.as_bytes())?;
    // ensure we have permissions
    check_can_send(&deps, env, &token)?;
    // set owner and remove existing approvals
    token.owner = deps.api.canonical_address(recipient)?;
    token.approvals = vec![];
    tokens(&mut deps.storage).save(token_id.as_bytes(), &token)?;
    Ok(token)
}

pub fn handle_approve<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    token_id: String,
    expires: Option<Expiration>,
) -> StdResult<HandleResponse> {
    _update_approvals(deps, &env, &spender, &token_id, true, expires)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "approve"),
            attr("sender", env.message.sender),
            attr("spender", spender),
            attr("token_id", token_id),
        ],
        data: None,
    })
}

pub fn handle_revoke<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    token_id: String,
) -> StdResult<HandleResponse> {
    _update_approvals(deps, &env, &spender, &token_id, false, None)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "revoke"),
            attr("sender", env.message.sender),
            attr("spender", spender),
            attr("token_id", token_id),
        ],
        data: None,
    })
}

pub fn _update_approvals<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: &Env,
    spender: &HumanAddr,
    token_id: &str,
    // if add == false, remove. if add == true, remove then set with this expiration
    add: bool,
    expires: Option<Expiration>,
) -> StdResult<TokenInfo> {
    let mut token = tokens(&mut deps.storage).load(token_id.as_bytes())?;
    // ensure we have permissions
    check_can_approve(&deps, &env, &token)?;

    // update the approval list (remove any for the same spender before adding)
    let spender_raw = deps.api.canonical_address(&spender)?;
    token.approvals = token
        .approvals
        .into_iter()
        .filter(|apr| apr.spender != spender_raw)
        .collect();

    // only difference between approve and revoke
    if add {
        // reject expired data as invalid
        let expires = expires.unwrap_or_default();
        if expires.is_expired(&env.block) {
            return Err(StdError::generic_err(
                "Cannot set approval that is already expired",
            ));
        }
        let approval = Approval {
            spender: spender_raw,
            expires,
        };
        token.approvals.push(approval);
    }

    tokens(&mut deps.storage).save(token_id.as_bytes(), &token)?;

    Ok(token)
}

pub fn handle_approve_all<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    operator: HumanAddr,
    expires: Option<Expiration>,
) -> StdResult<HandleResponse> {
    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(StdError::generic_err(
            "Cannot set approval that is already expired",
        ));
    }

    // set the operator for us
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    let operator_raw = deps.api.canonical_address(&operator)?;
    operators(&mut deps.storage, &sender_raw).save(operator_raw.as_slice(), &expires)?;

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "approve_all"),
            attr("sender", env.message.sender),
            attr("operator", operator),
        ],
        data: None,
    })
}

pub fn handle_revoke_all<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    operator: HumanAddr,
) -> StdResult<HandleResponse> {
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    let operator_raw = deps.api.canonical_address(&operator)?;
    operators(&mut deps.storage, &sender_raw).remove(operator_raw.as_slice());

    Ok(HandleResponse {
        messages: vec![],
        attributes: vec![
            attr("action", "revoke_all"),
            attr("sender", env.message.sender),
            attr("operator", operator),
        ],
        data: None,
    })
}

/// returns true iff the sender can execute approve or reject on the contract
fn check_can_approve<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
    token: &TokenInfo,
) -> StdResult<()> {
    // owner can approve
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    if token.owner == sender_raw {
        return Ok(());
    }
    // operator can approve
    let op = operators_read(&deps.storage, &token.owner).may_load(sender_raw.as_slice())?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(StdError::unauthorized())
            } else {
                Ok(())
            }
        }
        None => Err(StdError::unauthorized()),
    }
}

/// returns true iff the sender can transfer ownership of the token
fn check_can_send<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    env: &Env,
    token: &TokenInfo,
) -> StdResult<()> {
    // owner can send
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    if token.owner == sender_raw {
        return Ok(());
    }

    // any non-expired token approval can send
    if token
        .approvals
        .iter()
        .any(|apr| apr.spender == sender_raw && !apr.expires.is_expired(&env.block))
    {
        return Ok(());
    }

    // operator can send
    let op = operators_read(&deps.storage, &token.owner).may_load(sender_raw.as_slice())?;
    match op {
        Some(ex) => {
            if ex.is_expired(&env.block) {
                Err(StdError::unauthorized())
            } else {
                Ok(())
            }
        }
        None => Err(StdError::unauthorized()),
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Minter {} => to_binary(&query_minter(deps)?),
        QueryMsg::ContractInfo {} => to_binary(&query_contract_info(deps)?),
        QueryMsg::NftInfo { token_id } => to_binary(&query_nft_info(deps, token_id)?),
        QueryMsg::OwnerOf { token_id } => to_binary(&query_owner_of(deps, token_id)?),
        QueryMsg::AllNftInfo { token_id } => to_binary(&query_all_nft_info(deps, token_id)?),
        QueryMsg::ApprovedForAll {
            owner,
            start_after,
            limit,
        } => to_binary(&query_all_approvals(deps, owner, start_after, limit)?),
        QueryMsg::NumTokens {} => to_binary(&query_num_tokens(deps)?),
        QueryMsg::AllTokens { start_after, limit } => {
            to_binary(&query_all_tokens(deps, start_after, limit)?)
        }
    }
}

fn query_minter<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<MinterResponse> {
    let minter_raw = mint_read(&deps.storage).load()?;
    let minter = deps.api.human_address(&minter_raw)?;
    Ok(MinterResponse { minter })
}

fn query_contract_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ContractInfoResponse> {
    contract_info_read(&deps.storage).load()
}

fn query_num_tokens<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<NumTokensResponse> {
    let count = num_tokens(&deps.storage)?;
    Ok(NumTokensResponse { count })
}

fn query_nft_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: String,
) -> StdResult<NftInfoResponse> {
    let info = tokens_read(&deps.storage).load(token_id.as_bytes())?;
    Ok(NftInfoResponse {
        name: info.name,
        description: info.description,
        image: info.image,
    })
}

fn query_owner_of<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: String,
) -> StdResult<OwnerOfResponse> {
    let info = tokens_read(&deps.storage).load(token_id.as_bytes())?;
    Ok(OwnerOfResponse {
        owner: deps.api.human_address(&info.owner)?,
        approvals: humanize_approvals(deps.api, &info)?,
    })
}

const DEFAULT_LIMIT: u32 = 10;
const MAX_LIMIT: u32 = 30;

fn query_all_approvals<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    owner: HumanAddr,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<ApprovedForAllResponse> {
    let owner_raw = deps.api.canonical_address(&owner)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start_human(deps.api, start_after)?;

    let res: StdResult<Vec<_>> = operators_read(&deps.storage, &owner_raw)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            item.and_then(|(k, expires)| {
                Ok(cw721::Approval {
                    spender: deps.api.human_address(&k.into())?,
                    expires,
                })
            })
        })
        .collect();
    Ok(ApprovedForAllResponse { operators: res? })
}

fn query_all_tokens<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<TokensResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start_string(start_after);

    let tokens: StdResult<Vec<String>> = tokens_read(&deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| item.map(|(k, _)| String::from_utf8_lossy(&k).to_string()))
        .collect();
    Ok(TokensResponse { tokens: tokens? })
}

fn query_all_nft_info<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    token_id: String,
) -> StdResult<AllNftInfoResponse> {
    let info = tokens_read(&deps.storage).load(token_id.as_bytes())?;
    Ok(AllNftInfoResponse {
        access: OwnerOfResponse {
            owner: deps.api.human_address(&info.owner)?,
            approvals: humanize_approvals(deps.api, &info)?,
        },
        info: NftInfoResponse {
            name: info.name,
            description: info.description,
            image: info.image,
        },
    })
}

fn humanize_approvals<A: Api>(api: A, info: &TokenInfo) -> StdResult<Vec<cw721::Approval>> {
    info.approvals
        .iter()
        .map(|apr| humanize_approval(api, apr))
        .collect()
}

fn humanize_approval<A: Api>(api: A, approval: &Approval) -> StdResult<cw721::Approval> {
    Ok(cw721::Approval {
        spender: api.human_address(&approval.spender)?,
        expires: approval.expires,
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{StdError, WasmMsg};

    use super::*;
    use cw721::ApprovedForAllResponse;

    const MINTER: &str = "merlin";
    const CONTRACT_NAME: &str = "Magic Power";
    const SYMBOL: &str = "MGK";

    fn setup_contract<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>) {
        let msg = InitMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            minter: MINTER.into(),
        };
        let env = mock_env("creator", &[]);
        let res = init(deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
            minter: MINTER.into(),
        };
        let env = mock_env("creator", &[]);

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query_minter(&deps).unwrap();
        assert_eq!(MINTER, res.minter.as_str());
        let info = query_contract_info(&deps).unwrap();
        assert_eq!(
            info,
            ContractInfoResponse {
                name: CONTRACT_NAME.to_string(),
                symbol: SYMBOL.to_string(),
            }
        );

        let count = query_num_tokens(&deps).unwrap();
        assert_eq!(0, count.count);

        // list the token_ids
        let tokens = query_all_tokens(&deps, None, None).unwrap();
        assert_eq!(0, tokens.tokens.len());
    }

    #[test]
    fn minting() {
        let mut deps = mock_dependencies(20, &[]);
        setup_contract(&mut deps);

        let token_id = "petrify".to_string();
        let name = "Petrify with Gaze".to_string();
        let description = "Allows the owner to petrify anyone looking at him or her".to_string();

        let mint_msg = HandleMsg::Mint {
            token_id: token_id.clone(),
            owner: "medusa".into(),
            name: name.clone(),
            description: Some(description.clone()),
            image: None,
        };

        // random cannot mint
        let random = mock_env("random", &[]);
        let err = handle(&mut deps, random, mint_msg.clone()).unwrap_err();
        match err {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // minter can mint
        let allowed = mock_env(MINTER, &[]);
        let _ = handle(&mut deps, allowed, mint_msg.clone()).unwrap();

        // ensure num tokens increases
        let count = query_num_tokens(&deps).unwrap();
        assert_eq!(1, count.count);

        // unknown nft returns error
        let _ = query_nft_info(&deps, "unknown".to_string()).unwrap_err();

        // this nft info is correct
        let info = query_nft_info(&deps, token_id.clone()).unwrap();
        assert_eq!(
            info,
            NftInfoResponse {
                name: name.clone(),
                description: description.clone(),
                image: None,
            }
        );

        // owner info is correct
        let owner = query_owner_of(&deps, token_id.clone()).unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: "medusa".into(),
                approvals: vec![],
            }
        );

        // Cannot mint same token_id again
        let mint_msg2 = HandleMsg::Mint {
            token_id: token_id.clone(),
            owner: "hercules".into(),
            name: "copy cat".into(),
            description: None,
            image: None,
        };

        let allowed = mock_env(MINTER, &[]);
        let err = handle(&mut deps, allowed, mint_msg2).unwrap_err();
        match err {
            StdError::GenericErr { msg, .. } => {
                assert_eq!(msg.as_str(), "token_id already claimed")
            }
            e => panic!("unexpected error: {}", e),
        }

        // list the token_ids
        let tokens = query_all_tokens(&deps, None, None).unwrap();
        assert_eq!(1, tokens.tokens.len());
        assert_eq!(vec![token_id], tokens.tokens);
    }

    #[test]
    fn transferring_nft() {
        let mut deps = mock_dependencies(20, &[]);
        setup_contract(&mut deps);

        // Mint a token
        let token_id = "melt".to_string();
        let name = "Melting power".to_string();
        let description = "Allows the owner to melt anyone looking at him or her".to_string();

        let mint_msg = HandleMsg::Mint {
            token_id: token_id.clone(),
            owner: "venus".into(),
            name: name.clone(),
            description: Some(description.clone()),
            image: None,
        };

        let minter = mock_env(MINTER, &[]);
        handle(&mut deps, minter, mint_msg).unwrap();

        // random cannot transfer
        let random = mock_env("random", &[]);
        let transfer_msg = HandleMsg::TransferNft {
            recipient: "random".into(),
            token_id: token_id.clone(),
        };

        let err = handle(&mut deps, random, transfer_msg.clone()).unwrap_err();

        match err {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // owner can
        let random = mock_env("venus", &[]);
        let transfer_msg = HandleMsg::TransferNft {
            recipient: "random".into(),
            token_id: token_id.clone(),
        };

        let res = handle(&mut deps, random, transfer_msg.clone()).unwrap();

        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "transfer_nft"),
                    attr("sender", "venus"),
                    attr("recipient", "random"),
                    attr("token_id", token_id),
                ],
                data: None,
            }
        );
    }

    #[test]
    fn sending_nft() {
        let mut deps = mock_dependencies(20, &[]);
        setup_contract(&mut deps);

        // Mint a token
        let token_id = "melt".to_string();
        let name = "Melting power".to_string();
        let description = "Allows the owner to melt anyone looking at him or her".to_string();

        let mint_msg = HandleMsg::Mint {
            token_id: token_id.clone(),
            owner: "venus".into(),
            name: name.clone(),
            description: Some(description.clone()),
            image: None,
        };

        let minter = mock_env(MINTER, &[]);
        handle(&mut deps, minter, mint_msg).unwrap();

        // random cannot send
        let inner_msg = WasmMsg::Execute {
            contract_addr: "another_contract".into(),
            msg: to_binary("You now have the melting power").unwrap(),
            send: vec![],
        };
        let msg: CosmosMsg = CosmosMsg::Wasm(inner_msg);

        let send_msg = HandleMsg::SendNft {
            contract: "another_contract".into(),
            token_id: token_id.clone(),
            msg: Some(to_binary(&msg).unwrap()),
        };

        let random = mock_env("random", &[]);
        let err = handle(&mut deps, random, send_msg.clone()).unwrap_err();
        match err {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but owner can
        let random = mock_env("venus", &[]);
        let res = handle(&mut deps, random, send_msg).unwrap();
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![msg],
                attributes: vec![
                    attr("action", "send_nft"),
                    attr("sender", "venus"),
                    attr("recipient", "another_contract"),
                    attr("token_id", token_id),
                ],
                data: None,
            }
        );
    }

    #[test]
    fn approving_revoking() {
        let mut deps = mock_dependencies(20, &[]);
        setup_contract(&mut deps);

        // Mint a token
        let token_id = "grow".to_string();
        let name = "Growing power".to_string();
        let description = "Allows the owner to grow anything".to_string();

        let mint_msg = HandleMsg::Mint {
            token_id: token_id.clone(),
            owner: "demeter".into(),
            name: name.clone(),
            description: Some(description.clone()),
            image: None,
        };

        let minter = mock_env(MINTER, &[]);
        handle(&mut deps, minter, mint_msg).unwrap();

        // Give random transferring power
        let approve_msg = HandleMsg::Approve {
            spender: "random".into(),
            token_id: token_id.clone(),
            expires: None,
        };
        let owner = mock_env("demeter", &[]);
        let res = handle(&mut deps, owner, approve_msg).unwrap();
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "approve"),
                    attr("sender", "demeter"),
                    attr("spender", "random"),
                    attr("token_id", token_id.clone()),
                ],
                data: None,
            }
        );

        // random can now transfer
        let random = mock_env("random", &[]);
        let transfer_msg = HandleMsg::TransferNft {
            recipient: "person".into(),
            token_id: token_id.clone(),
        };
        handle(&mut deps, random, transfer_msg).unwrap();

        // Approvals are removed / cleared
        let query_msg = QueryMsg::OwnerOf {
            token_id: token_id.clone(),
        };
        let res: OwnerOfResponse = from_binary(&query(&deps, query_msg.clone()).unwrap()).unwrap();
        assert_eq!(
            res,
            OwnerOfResponse {
                owner: "person".into(),
                approvals: vec![],
            }
        );

        // Approve, revoke, and check for empty, to test revoke
        let approve_msg = HandleMsg::Approve {
            spender: "random".into(),
            token_id: token_id.clone(),
            expires: None,
        };
        let owner = mock_env("person", &[]);
        handle(&mut deps, owner.clone(), approve_msg).unwrap();

        let revoke_msg = HandleMsg::Revoke {
            spender: "random".into(),
            token_id: token_id.clone(),
        };
        handle(&mut deps, owner, revoke_msg).unwrap();

        // Approvals are now removed / cleared
        let res: OwnerOfResponse = from_binary(&query(&deps, query_msg).unwrap()).unwrap();
        assert_eq!(
            res,
            OwnerOfResponse {
                owner: "person".into(),
                approvals: vec![],
            }
        );
    }

    #[test]
    fn approving_all_revoking_all() {
        let mut deps = mock_dependencies(20, &[]);
        setup_contract(&mut deps);

        // Mint a couple tokens (from the same owner)
        let token_id1 = "grow1".to_string();
        let name1 = "Growing power".to_string();
        let description1 = "Allows the owner the power to grow anything".to_string();
        let token_id2 = "grow2".to_string();
        let name2 = "More growing power".to_string();
        let description2 = "Allows the owner the power to grow anything even faster".to_string();

        let mint_msg1 = HandleMsg::Mint {
            token_id: token_id1.clone(),
            owner: "demeter".into(),
            name: name1.clone(),
            description: Some(description1.clone()),
            image: None,
        };

        let minter = mock_env(MINTER, &[]);
        handle(&mut deps, minter.clone(), mint_msg1).unwrap();

        let mint_msg2 = HandleMsg::Mint {
            token_id: token_id2.clone(),
            owner: "demeter".into(),
            name: name2.clone(),
            description: Some(description2.clone()),
            image: None,
        };

        handle(&mut deps, minter, mint_msg2).unwrap();

        // paginate the token_ids
        let tokens = query_all_tokens(&deps, None, Some(1)).unwrap();
        assert_eq!(1, tokens.tokens.len());
        assert_eq!(vec![token_id1.clone()], tokens.tokens);
        let tokens = query_all_tokens(&deps, Some(token_id1.clone()), Some(3)).unwrap();
        assert_eq!(1, tokens.tokens.len());
        assert_eq!(vec![token_id2.clone()], tokens.tokens);

        // demeter gives random full (operator) power over her tokens
        let approve_all_msg = HandleMsg::ApproveAll {
            operator: "random".into(),
            expires: None,
        };
        let owner = mock_env("demeter", &[]);
        let res = handle(&mut deps, owner, approve_all_msg).unwrap();
        assert_eq!(
            res,
            HandleResponse {
                messages: vec![],
                attributes: vec![
                    attr("action", "approve_all"),
                    attr("sender", "demeter"),
                    attr("operator", "random"),
                ],
                data: None,
            }
        );

        // random can now transfer
        let random = mock_env("random", &[]);
        let transfer_msg = HandleMsg::TransferNft {
            recipient: "person".into(),
            token_id: token_id1.clone(),
        };
        handle(&mut deps, random.clone(), transfer_msg).unwrap();

        // random can now send
        let inner_msg = WasmMsg::Execute {
            contract_addr: "another_contract".into(),
            msg: to_binary("You now also have the growing power").unwrap(),
            send: vec![],
        };
        let msg: CosmosMsg = CosmosMsg::Wasm(inner_msg);

        let send_msg = HandleMsg::SendNft {
            contract: "another_contract".into(),
            token_id: token_id2.clone(),
            msg: Some(to_binary(&msg).unwrap()),
        };
        handle(&mut deps, random, send_msg).unwrap();

        // Approve_all, revoke_all, and check for empty, to test revoke_all
        let approve_all_msg = HandleMsg::ApproveAll {
            operator: "operator".into(),
            expires: None,
        };
        // person is now the owner of the tokens
        let owner = mock_env("person", &[]);
        handle(&mut deps, owner.clone(), approve_all_msg).unwrap();

        let res = query_all_approvals(&deps, "person".into(), None, None).unwrap();
        assert_eq!(
            res,
            ApprovedForAllResponse {
                operators: vec![cw721::Approval {
                    spender: "operator".into(),
                    expires: Expiration::Never {}
                }]
            }
        );

        // second approval
        let buddy_expires = Expiration::AtHeight(1234567);
        let approve_all_msg = HandleMsg::ApproveAll {
            operator: "buddy".into(),
            expires: Some(buddy_expires),
        };
        let owner = mock_env("person", &[]);
        handle(&mut deps, owner.clone(), approve_all_msg).unwrap();

        // and paginate queries
        let res = query_all_approvals(&deps, "person".into(), None, Some(1)).unwrap();
        assert_eq!(
            res,
            ApprovedForAllResponse {
                operators: vec![cw721::Approval {
                    spender: "buddy".into(),
                    expires: buddy_expires,
                }]
            }
        );
        let res =
            query_all_approvals(&deps, "person".into(), Some("buddy".into()), Some(2)).unwrap();
        assert_eq!(
            res,
            ApprovedForAllResponse {
                operators: vec![cw721::Approval {
                    spender: "operator".into(),
                    expires: Expiration::Never {}
                }]
            }
        );

        let revoke_all_msg = HandleMsg::RevokeAll {
            operator: "operator".into(),
        };
        handle(&mut deps, owner, revoke_all_msg).unwrap();

        // Approvals are removed / cleared without affecting others
        let res = query_all_approvals(&deps, "person".into(), None, None).unwrap();
        assert_eq!(
            res,
            ApprovedForAllResponse {
                operators: vec![cw721::Approval {
                    spender: "buddy".into(),
                    expires: buddy_expires,
                }]
            }
        );
    }
}
