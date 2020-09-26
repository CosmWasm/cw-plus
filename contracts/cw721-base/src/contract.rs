use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage,
};
use cw2::set_contract_version;

use crate::msg::{HandleMsg, InitMsg, MinterResponse, QueryMsg};
use crate::state::{
    contract_info, contract_info_read, increment_tokens, mint, mint_read, num_tokens,
    operators_read, tokens, tokens_read, Approval, TokenInfo,
};
use cw721::{
    AllNftInfoResponse, ContractInfoResponse, Expiration, NftInfoResponse, NumTokensResponse,
    OwnerOfResponse,
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
        _ => panic!("not implemented"),
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
    let minter_human = deps.api.human_address(&minter)?;

    if minter_human != env.message.sender {
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
    tokens(&mut deps.storage).save(token_id.as_bytes(), &token)?;

    increment_tokens(&mut deps.storage)?;

    // TODO: set logs
    Ok(HandleResponse::default())
}

pub fn handle_approve<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    token_id: String,
    expires: Option<Expiration>,
) -> StdResult<HandleResponse> {
    // reject expired data as invalid
    let expires = expires.unwrap_or_default();
    if expires.is_expired(&env.block) {
        return Err(StdError::generic_err(
            "Cannot set approval that is already expired",
        ));
    }

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
    let approval = Approval {
        spender: spender_raw,
        expires,
    };
    token.approvals.push(approval);
    tokens(&mut deps.storage).save(token_id.as_bytes(), &token)?;

    // TODO: set logs
    Ok(HandleResponse::default())
}

pub fn handle_revoke<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: HumanAddr,
    token_id: String,
) -> StdResult<HandleResponse> {
    let mut token = tokens(&mut deps.storage).load(token_id.as_bytes())?;
    // ensure we have permissions
    check_can_approve(&deps, &env, &token)?;

    // remove this spender from the list
    let spender_raw = deps.api.canonical_address(&spender)?;
    token.approvals = token
        .approvals
        .into_iter()
        .filter(|apr| apr.spender != spender_raw)
        .collect();
    tokens(&mut deps.storage).save(token_id.as_bytes(), &token)?;

    // TODO: set logs
    Ok(HandleResponse::default())
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
        QueryMsg::ApprovedForAll { owner: _ } => panic!("not implemented"),
        QueryMsg::NumTokens {} => to_binary(&query_num_tokens(deps)?),
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
        expires: approval.expires.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::StdError;

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
                image: None
            }
        );

        // owner info is correct
        let owner = query_owner_of(&deps, token_id.clone()).unwrap();
        assert_eq!(
            owner,
            OwnerOfResponse {
                owner: "medusa".into(),
                approvals: vec![]
            }
        );
    }
}
