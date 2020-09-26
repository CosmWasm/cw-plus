use cosmwasm_std::{
    to_binary, Api, Binary, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    StdError, StdResult, Storage,
};
use cw2::set_contract_version;

use crate::msg::{HandleMsg, InitMsg, MinterResponse, QueryMsg};
use crate::state::{
    contract_info, contract_info_read, mint, mint_read, tokens, tokens_read, Approval, TokenInfo,
};
use cw721::{AllNftInfoResponse, ContractInfoResponse, NftInfoResponse, OwnerOfResponse};

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

    // TODO: set logs
    Ok(HandleResponse::default())
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
        QueryMsg::NumTokens {} => panic!("not implemented"),
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
    use cosmwasm_std::{coins, from_binary, StdError};

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(20, &[]);

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(1000, "earth"));

        // we can just call .unwrap() to assert this was a success
        let res = init(&mut deps, env, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(&deps, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(17, value.count);
    }

    #[test]
    fn increment() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        // beneficiary can release it
        let env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::Increment {};
        let _res = handle(&mut deps, env, msg).unwrap();

        // should increase counter by 1
        let res = query(&deps, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(18, value.count);
    }

    #[test]
    fn reset() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let msg = InitMsg { count: 17 };
        let env = mock_env("creator", &coins(2, "token"));
        let _res = init(&mut deps, env, msg).unwrap();

        // beneficiary can release it
        let unauth_env = mock_env("anyone", &coins(2, "token"));
        let msg = HandleMsg::Reset { count: 5 };
        let res = handle(&mut deps, unauth_env, msg);
        match res {
            Err(StdError::Unauthorized { .. }) => {}
            _ => panic!("Must return unauthorized error"),
        }

        // only the original creator can reset the counter
        let auth_env = mock_env("creator", &coins(2, "token"));
        let msg = HandleMsg::Reset { count: 5 };
        let _res = handle(&mut deps, auth_env, msg).unwrap();

        // should now be 5
        let res = query(&deps, QueryMsg::GetCount {}).unwrap();
        let value: CountResponse = from_binary(&res).unwrap();
        assert_eq!(5, value.count);
    }
}
