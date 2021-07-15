#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, SubMsg,
    Uint128, WasmMsg,
};

use crate::state::{Config, CLAIM, CONFIG, MERKLE_ROOT, STAGE};

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, IsClaimedResponse, LatestStageResponse,
    MerkleRootResponse, MigrateMsg, QueryMsg,
};
use cw20::Cw20ExecuteMsg;
use cw_storage_plus::U8Key;
use hex;
use sha3::Digest;
use std::cmp::Ordering;
use std::convert::TryInto;

// Version info, for migration info
const _CONTRACT_NAME: &str = "crates.io:cw20-merkle-airdrop";
const _CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let config = Config {
        owner: deps.api.addr_validate(&msg.owner)?,
        cw20_token_address: deps.api.addr_validate(&msg.cw20_token_address)?,
    };
    CONFIG.save(deps.storage, &config)?;

    let stage: u8 = 0;
    STAGE.save(deps.storage, &stage)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig { owner } => execute_update_config(deps, env, info, owner),
        ExecuteMsg::RegisterMerkleRoot { merkle_root } => {
            execute_register_merkle_root(deps, env, info, merkle_root)
        }
        ExecuteMsg::Claim {
            stage,
            amount,
            proof,
        } => execute_claim(deps, env, info, stage, amount, proof),
    }
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Option<String>,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    // validate owner change
    let new_owner = owner
        .ok_or(ContractError::InvalidInput {})
        .and_then(|o| deps.api.addr_validate(o.as_str()).map_err(|err| err.into()))?;

    CONFIG.update(deps.storage, |mut exists| -> StdResult<_> {
        exists.owner = new_owner;
        Ok(exists)
    })?;

    Ok(Response {
        messages: vec![],
        attributes: vec![attr("action", "update_config")],
        data: None,
        events: vec![],
    })
}

pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: String,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender != cfg.owner {
        return Err(ContractError::Unauthorized {});
    }

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root.to_string(), &mut root_buf)?;

    let latest_stage: u8 = STAGE.load(deps.storage)?;
    let stage = latest_stage + 1;

    MERKLE_ROOT.save(deps.storage, U8Key::from(stage), &merkle_root)?;
    STAGE.save(deps.storage, &stage)?;

    Ok(Response {
        messages: vec![],
        attributes: vec![
            attr("action", "register_merkle_root"),
            attr("stage", stage),
            attr("merkle_root", merkle_root),
        ],
        data: None,
        events: vec![],
    })
}

pub fn execute_claim(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
) -> Result<Response, ContractError> {
    // verify not claimed
    let claimed = CLAIM.may_load(deps.storage, (&info.sender, U8Key::from(stage)))?;
    if claimed.is_some() {
        return Err(ContractError::Claimed {});
    }

    let config = CONFIG.load(deps.storage)?;
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage.into())?;

    let user_input: String = info.sender.to_string() + &amount.to_string();
    let mut hash: [u8; 32] = sha3::Keccak256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    for p in proof {
        let mut proof_buf: [u8; 32] = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;

        hash = match bytes_cmp(hash, proof_buf) {
            Ordering::Less => sha3::Keccak256::digest(&[hash, proof_buf].concat())
                .as_slice()
                .try_into()
                .map_err(|_| ContractError::WrongLength {}),
            _ => sha3::Keccak256::digest(&[proof_buf, hash].concat())
                .as_slice()
                .try_into()
                .map_err(|_| ContractError::WrongLength {}),
        }?;
    }

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)?;
    if root_buf != hash {
        return Err(ContractError::VerificationFailed {});
    }

    // Update claim index to the current stage
    CLAIM.save(deps.storage, (&info.sender, stage.into()), &true)?;

    let msgs: Vec<SubMsg> = vec![SubMsg::new(WasmMsg::Execute {
        contract_addr: config.cw20_token_address.to_string(),
        funds: vec![],
        msg: to_binary(&Cw20ExecuteMsg::Transfer {
            recipient: info.sender.to_string(),
            amount,
        })?,
    })];
    Ok(Response {
        messages: msgs,
        attributes: vec![
            attr("action", "claim"),
            attr("stage", stage),
            attr("address", info.sender),
            attr("amount", amount),
        ],
        data: None,
        events: vec![],
    })
}

fn bytes_cmp(a: [u8; 32], b: [u8; 32]) -> std::cmp::Ordering {
    let mut i = 0;
    while i < 32 {
        match a[i].cmp(&b[i]) {
            Ordering::Less => return Ordering::Less,
            Ordering::Greater => return std::cmp::Ordering::Greater,
            Ordering::Equal => i += 1,
        }
    }

    std::cmp::Ordering::Equal
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => to_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            to_binary(&query_is_claimed(deps, stage, address)?)
        }
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.to_string(),
        cw20_token_address: cfg.cw20_token_address.to_string(),
    })
}

pub fn query_merkle_root(deps: Deps, stage: u8) -> StdResult<MerkleRootResponse> {
    let merkle_root = MERKLE_ROOT.load(deps.storage, U8Key::from(stage))?;
    let resp = MerkleRootResponse { stage, merkle_root };

    Ok(resp)
}

pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = STAGE.load(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_is_claimed(deps: Deps, stage: u8, address: String) -> StdResult<IsClaimedResponse> {
    let key: (&Addr, U8Key) = (&deps.api.addr_validate(&address)?, stage.into());
    let is_claimed = CLAIM.load(deps.storage, key)?;
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::{from_binary, OwnedDeps, MemoryStorage};
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};

    fn setup_test_case() -> (OwnedDeps<MemoryStorage, MockApi, MockQuerier>, Env) {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: "owner0000".to_string(),
            cw20_token_address: "anchor0000".to_string(),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);

        let _ = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();
        (deps, env)
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies(&[]);

        let msg = InstantiateMsg {
            owner: "owner0000".to_string(),
            cw20_token_address: "anchor0000".to_string(),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let _ = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // it worked, let's query the state
        let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
        //let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner0000", config.owner.as_str());
        assert_eq!("anchor0000", config.cw20_token_address.as_str());

        let res = query(deps.as_ref(), env, QueryMsg::LatestStage {}).unwrap();
        let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
        assert_eq!(0u8, latest_stage.latest_stage);
    }

    #[test]
    fn test_update_config() {
        let (mut deps, env) = setup_test_case();

        // it worked, let's query the state
        let config: ConfigResponse = query_config(deps.as_ref()).unwrap();
        //let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner0000", config.owner.as_str());
        assert_eq!("anchor0000", config.cw20_token_address.as_str());

        let new_owner = "owner0000";
        let msg = ExecuteMsg::UpdateConfig { owner: Some(new_owner.to_string()) };

        // random cannot update
        let info = mock_info("random", &[]);
        let resp = execute(deps.as_mut(), env.clone(), info, msg.clone());
        match resp {
            Ok(_) => panic!("expected error"),
            Err(_) => {}
        }

        // owner can update
        let info = mock_info("owner0000", &[]);
        let resp = execute(deps.as_mut(), env, info, msg);
        match resp {
            Ok(_) => {},
            Err(_) => panic!("expected ok")
        }
    }
}
