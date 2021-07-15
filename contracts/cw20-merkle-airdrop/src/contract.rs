use cosmwasm_std::{log, to_binary, Api, Binary, CosmosMsg, Env, Extern, HandleResponse, StdResult, Response, HumanAddr, InitResponse, InitResult, MigrateResponse, MigrateResult, Querier, StdError, Storage, Uint128, WasmMsg, DepsMut, MessageInfo, attr};

use crate::state::{read_claimed, read_config, read_latest_stage, read_merkle_root, store_claimed, store_config, store_latest_stage, store_merkle_root, Config, STAGE, CONFIG_KEY, CONFIG, MERKLE_ROOT};

use cw20::Cw20ExecuteMsg;
use hex;
use sha3::Digest;
use std::convert::TryInto;
use crate::msg::{InstantiateMsg, ExecuteMsg};
use std::borrow::Borrow;
use crate::error::ContractError;
use cw_storage_plus::U8Key;

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-merkle-airdrop";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InstantiateMsg) -> StdResult<Response> {
    let config = &Config {
            owner: deps.api.addr_validate(&msg.owner)?,
            cw20_token_address: deps.api.addr_validate(&msg.cw20_token_address)?,
        };
    CONFIG.save(deps.storage, config)?;

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
        } => claim(deps, env, stage, amount, proof),
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
        return Err(ContractError::Unauthorized {})
    }

    // validate owner change
    let new_owner = owner
        .ok_or(ContractError::InvalidInput {})
        .and_then(|o| deps.api.addr_validate(o.as_str()))?;


    CONFIG.update(deps.storage, |mut exists| {
        exists.owner = new_owner;
        Ok(exists)
    })?;

    Ok(Response {
        messages: vec![],
        attributes: vec![attr("action", "update_config")],
        data: None,
        events: vec![]
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
        return Err(ContractError::Unauthorized {})
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
        events: vec![]
    })
}

pub fn claim<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
) -> StdResult<HandleResponse> {
    let config: Config = read_config(&deps.storage)?;
    let merkle_root: String = read_merkle_root(&deps.storage, stage)?;

    let user_raw = deps.api.canonical_address(&env.message.sender)?;

    // If user claimed target stage, return err
    if read_claimed(&deps.storage, &user_raw, stage)? {
        return Err(StdError::generic_err("Already claimed"));
    }

    let user_input: String = env.message.sender.to_string() + &amount.to_string();
    let mut hash: [u8; 32] = sha3::Keccak256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .expect("Wrong length");

    for p in proof {
        let mut proof_buf: [u8; 32] = [0; 32];
        match hex::decode_to_slice(p, &mut proof_buf) {
            Ok(()) => {}
            _ => return Err(StdError::generic_err("Invalid hex encoded proof")),
        }

        hash = if bytes_cmp(hash, proof_buf) == std::cmp::Ordering::Less {
            sha3::Keccak256::digest(&[hash, proof_buf].concat())
                .as_slice()
                .try_into()
                .expect("Wrong length")
        } else {
            sha3::Keccak256::digest(&[proof_buf, hash].concat())
                .as_slice()
                .try_into()
                .expect("Wrong length")
        };
    }

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf).unwrap();
    if root_buf != hash {
        return Err(StdError::generic_err("Verification is failed"));
    }

    // Update claim index to the current stage
    store_claimed(&mut deps.storage, &user_raw, stage)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.cw20_token_address)?,
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: env.message.sender.clone(),
                amount,
            })?,
        })],
        log: vec![
            log("action", "claim"),
            log("stage", stage),
            log("address", env.message.sender),
            log("amount", amount),
        ],
        data: None,
    })
}

fn bytes_cmp(a: [u8; 32], b: [u8; 32]) -> std::cmp::Ordering {
    let mut i = 0;
    while i < 32 {
        if a[i] > b[i] {
            return std::cmp::Ordering::Greater;
        } else if a[i] < b[i] {
            return std::cmp::Ordering::Less;
        }

        i += 1;
    }

    return std::cmp::Ordering::Equal;
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => to_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            to_binary(&query_is_claimed(deps, stage, address)?)
        }
    }
}

pub fn query_config<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<ConfigResponse> {
    let state = read_config(&deps.storage)?;
    let resp = ConfigResponse {
        owner: deps.api.human_address(&state.owner)?,
        anchor_token: deps.api.human_address(&state.cw20_token_address)?,
    };

    Ok(resp)
}

pub fn query_merkle_root<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    stage: u8,
) -> StdResult<MerkleRootResponse> {
    let merkle_root = read_merkle_root(&deps.storage, stage)?;
    let resp = MerkleRootResponse {
        stage: stage,
        merkle_root: merkle_root,
    };

    Ok(resp)
}

pub fn query_latest_stage<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
) -> StdResult<LatestStageResponse> {
    let latest_stage = read_latest_stage(&deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_is_claimed<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    stage: u8,
    address: HumanAddr,
) -> StdResult<IsClaimedResponse> {
    let user_raw = deps.api.canonical_address(&address)?;
    let resp = IsClaimedResponse {
        is_claimed: read_claimed(&deps.storage, &user_raw, stage)?,
    };

    Ok(resp)
}

pub fn migrate<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: MigrateMsg,
) -> MigrateResult {
    Ok(MigrateResponse::default())
}
