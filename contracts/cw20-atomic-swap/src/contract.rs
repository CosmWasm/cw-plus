#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Addr, BankMsg, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, WasmMsg,
};
use sha2::{Digest, Sha256};

use cw2::set_contract_version;
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg};

use crate::error::ContractError;
use crate::msg::{
    is_valid_name, BalanceHuman, CreateMsg, DetailsResponse, ExecuteMsg, InstantiateMsg,
    ListResponse, QueryMsg, ReceiveMsg,
};
use crate::state::{all_swap_ids, AtomicSwap, SWAPS};
use cw_storage_plus::Bound;

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-atomic-swap";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // No setup
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
        ExecuteMsg::Create(msg) => {
            let sent_funds = info.funds.clone();
            execute_create(deps, env, info, msg, Balance::from(sent_funds))
        }
        ExecuteMsg::Release { id, preimage } => execute_release(deps, env, id, preimage),
        ExecuteMsg::Refund { id } => execute_refund(deps, env, id),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
    }
}

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;
    let token = Cw20CoinVerified {
        address: info.sender,
        amount: wrapper.amount,
    };
    // we need to update the info... so the original sender is the one authorizing with these tokens
    let orig_info = MessageInfo {
        sender: deps.api.addr_validate(&wrapper.sender)?,
        funds: info.funds,
    };
    match msg {
        ReceiveMsg::Create(create) => {
            execute_create(deps, env, orig_info, create, Balance::Cw20(token))
        }
    }
}

pub fn execute_create(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: CreateMsg,
    balance: Balance,
) -> Result<Response, ContractError> {
    if !is_valid_name(&msg.id) {
        return Err(ContractError::InvalidId {});
    }

    // this ignores 0 value coins, must have one or more with positive balance
    if balance.is_empty() {
        return Err(ContractError::EmptyBalance {});
    }

    // Ensure this is 32 bytes hex-encoded, and decode
    let hash = parse_hex_32(&msg.hash)?;

    if msg.expires.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    let recipient = deps.api.addr_validate(&msg.recipient)?;

    let swap = AtomicSwap {
        hash: Binary(hash),
        recipient,
        source: info.sender,
        expires: msg.expires,
        balance,
    };

    // Try to store it, fail if the id already exists (unmodifiable swaps)
    SWAPS.update(deps.storage, &msg.id, |existing| match existing {
        None => Ok(swap),
        Some(_) => Err(ContractError::AlreadyExists {}),
    })?;

    let res = Response::new()
        .add_attribute("action", "create")
        .add_attribute("id", msg.id)
        .add_attribute("hash", msg.hash)
        .add_attribute("recipient", msg.recipient);
    Ok(res)
}

pub fn execute_release(
    deps: DepsMut,
    env: Env,
    id: String,
    preimage: String,
) -> Result<Response, ContractError> {
    let swap = SWAPS.load(deps.storage, &id)?;
    if swap.is_expired(&env.block) {
        return Err(ContractError::Expired {});
    }

    let hash = Sha256::digest(&parse_hex_32(&preimage)?);
    if hash.as_slice() != swap.hash.as_slice() {
        return Err(ContractError::InvalidPreimage {});
    }

    // Delete the swap
    SWAPS.remove(deps.storage, &id);

    // Send all tokens out
    let msgs = send_tokens(&swap.recipient, swap.balance)?;
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "release")
        .add_attribute("id", id)
        .add_attribute("preimage", preimage)
        .add_attribute("to", swap.recipient.to_string()))
}

pub fn execute_refund(deps: DepsMut, env: Env, id: String) -> Result<Response, ContractError> {
    let swap = SWAPS.load(deps.storage, &id)?;
    // Anyone can try to refund, as long as the contract is expired
    if !swap.is_expired(&env.block) {
        return Err(ContractError::NotExpired {});
    }

    // We delete the swap
    SWAPS.remove(deps.storage, &id);

    let msgs = send_tokens(&swap.source, swap.balance)?;
    Ok(Response::new()
        .add_submessages(msgs)
        .add_attribute("action", "refund")
        .add_attribute("id", id)
        .add_attribute("to", swap.source.to_string()))
}

fn parse_hex_32(data: &str) -> Result<Vec<u8>, ContractError> {
    match hex::decode(data) {
        Ok(bin) => {
            if bin.len() == 32 {
                Ok(bin)
            } else {
                Err(ContractError::InvalidHash(bin.len() * 2))
            }
        }
        Err(e) => Err(ContractError::ParseError(e.to_string())),
    }
}

fn send_tokens(to: &Addr, amount: Balance) -> StdResult<Vec<SubMsg>> {
    if amount.is_empty() {
        Ok(vec![])
    } else {
        match amount {
            Balance::Native(coins) => {
                let msg = BankMsg::Send {
                    to_address: to.into(),
                    amount: coins.into_vec(),
                };
                Ok(vec![SubMsg::new(msg)])
            }
            Balance::Cw20(coin) => {
                let msg = Cw20ExecuteMsg::Transfer {
                    recipient: to.into(),
                    amount: coin.amount,
                };
                let exec = WasmMsg::Execute {
                    contract_addr: coin.address.into(),
                    msg: to_binary(&msg)?,
                    funds: vec![],
                };
                Ok(vec![SubMsg::new(exec)])
            }
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::List { start_after, limit } => to_binary(&query_list(deps, start_after, limit)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
    }
}

fn query_details(deps: Deps, id: String) -> StdResult<DetailsResponse> {
    let swap = SWAPS.load(deps.storage, &id)?;

    // Convert balance to human balance
    let balance_human = match swap.balance {
        Balance::Native(coins) => BalanceHuman::Native(coins.into_vec()),
        Balance::Cw20(coin) => BalanceHuman::Cw20(Cw20Coin {
            address: coin.address.into(),
            amount: coin.amount,
        }),
    };

    let details = DetailsResponse {
        id,
        hash: hex::encode(swap.hash.as_slice()),
        recipient: swap.recipient.into(),
        source: swap.source.into(),
        expires: swap.expires,
        balance: balance_human,
    };
    Ok(details)
}

// Settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

fn query_list(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|s| Bound::exclusive(s.as_bytes()));

    Ok(ListResponse {
        swaps: all_swap_ids(deps.storage, start, limit)?,
    })
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, from_binary, StdError, Timestamp, Uint128};

    use cw20::Expiration;

    use super::*;

    fn preimage() -> String {
        hex::encode(b"This is a string, 32 bytes long.")
    }

    fn custom_preimage(int: u16) -> String {
        hex::encode(format!("This is a custom string: {:>7}", int))
    }

    fn real_hash() -> String {
        hex::encode(&Sha256::digest(&hex::decode(preimage()).unwrap()))
    }

    fn custom_hash(int: u16) -> String {
        hex::encode(&Sha256::digest(&hex::decode(custom_preimage(int)).unwrap()))
    }

    fn mock_env_height(height: u64) -> Env {
        let mut env = mock_env();
        env.block.height = height;
        env
    }

    #[test]
    fn test_instantiate() {
        let mut deps = mock_dependencies();

        // Instantiate an empty contract
        let instantiate_msg = InstantiateMsg {};
        let info = mock_info("anyone", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        instantiate(deps.as_mut(), mock_env(), info, InstantiateMsg {}).unwrap();

        let sender = String::from("sender0001");
        let balance = coins(100, "tokens");

        // Cannot create, invalid ids
        let info = mock_info(&sender, &balance);
        for id in &["sh", "atomic_swap_id_too_long"] {
            let create = CreateMsg {
                id: id.to_string(),
                hash: real_hash(),
                recipient: String::from("rcpt0001"),
                expires: Expiration::AtHeight(123456),
            };
            let err = execute(
                deps.as_mut(),
                mock_env(),
                info.clone(),
                ExecuteMsg::Create(create.clone()),
            )
            .unwrap_err();
            assert_eq!(err, ContractError::InvalidId {});
        }

        // Cannot create, no funds
        let info = mock_info(&sender, &[]);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtHeight(123456),
        };
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Create(create)).unwrap_err();
        assert_eq!(err, ContractError::EmptyBalance {});

        // Cannot create, expired
        let info = mock_info(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtTime(Timestamp::from_seconds(1)),
        };
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Create(create)).unwrap_err();
        assert_eq!(err, ContractError::Expired {});

        // Cannot create, invalid hash
        let info = mock_info(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: "bu115h17".to_string(),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtHeight(123456),
        };
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Create(create)).unwrap_err();
        assert_eq!(
            err,
            ContractError::ParseError("Invalid character \'u\' at position 1".into())
        );

        // Can create, all valid
        let info = mock_info(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtHeight(123456),
        };
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Create(create)).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // Cannot re-create (modify), already existing
        let new_balance = coins(1, "tokens");
        let info = mock_info(&sender, &new_balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtHeight(123456),
        };
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Create(create)).unwrap_err();
        assert_eq!(err, ContractError::AlreadyExists {});
    }

    #[test]
    fn test_release() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        instantiate(deps.as_mut(), mock_env(), info, InstantiateMsg {}).unwrap();

        let sender = String::from("sender0001");
        let balance = coins(1000, "tokens");

        let info = mock_info(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtHeight(123456),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Create(create.clone()),
        )
        .unwrap();

        // Anyone can attempt release
        let info = mock_info("somebody", &[]);

        // Cannot release, wrong id
        let release = ExecuteMsg::Release {
            id: "swap0002".to_string(),
            preimage: preimage(),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), release).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));

        // Cannot release, invalid hash
        let release = ExecuteMsg::Release {
            id: "swap0001".to_string(),
            preimage: "bu115h17".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), release).unwrap_err();
        assert_eq!(
            err,
            ContractError::ParseError("Invalid character \'u\' at position 1".to_string())
        );

        // Cannot release, wrong hash
        let release = ExecuteMsg::Release {
            id: "swap0001".to_string(),
            preimage: hex::encode(b"This is 32 bytes, but incorrect."),
        };
        let err = execute(deps.as_mut(), mock_env(), info, release).unwrap_err();
        assert!(matches!(err, ContractError::InvalidPreimage {}));

        // Cannot release, expired
        let env = mock_env_height(123457);
        let info = mock_info("somebody", &[]);
        let release = ExecuteMsg::Release {
            id: "swap0001".to_string(),
            preimage: preimage(),
        };
        let err = execute(deps.as_mut(), env, info, release).unwrap_err();
        assert!(matches!(err, ContractError::Expired));

        // Can release, valid id, valid hash, and not expired
        let info = mock_info("somebody", &[]);
        let release = ExecuteMsg::Release {
            id: "swap0001".to_string(),
            preimage: preimage(),
        };
        let res = execute(deps.as_mut(), mock_env(), info.clone(), release.clone()).unwrap();
        assert_eq!(("action", "release"), res.attributes[0]);
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg::new(BankMsg::Send {
                to_address: create.recipient,
                amount: balance,
            })
        );

        // Cannot release again
        let err = execute(deps.as_mut(), mock_env(), info, release).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
    }

    #[test]
    fn test_refund() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        instantiate(deps.as_mut(), mock_env(), info, InstantiateMsg {}).unwrap();

        let sender = String::from("sender0001");
        let balance = coins(1000, "tokens");

        let info = mock_info(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtHeight(123456),
        };
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Create(create)).unwrap();

        // Anyone can attempt refund
        let info = mock_info("somebody", &[]);

        // Cannot refund, wrong id
        let refund = ExecuteMsg::Refund {
            id: "swap0002".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info.clone(), refund).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));

        // Cannot refund, not expired yet
        let refund = ExecuteMsg::Refund {
            id: "swap0001".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, refund).unwrap_err();
        assert!(matches!(err, ContractError::NotExpired { .. }));

        // Anyone can refund, if already expired
        let env = mock_env_height(123457);
        let info = mock_info("somebody", &[]);
        let refund = ExecuteMsg::Refund {
            id: "swap0001".to_string(),
        };
        let res = execute(deps.as_mut(), env.clone(), info.clone(), refund.clone()).unwrap();
        assert_eq!(("action", "refund"), res.attributes[0]);
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg::new(BankMsg::Send {
                to_address: sender,
                amount: balance,
            })
        );

        // Cannot refund again
        let err = execute(deps.as_mut(), env, info, refund).unwrap_err();
        assert!(matches!(err, ContractError::Std(StdError::NotFound { .. })));
    }

    #[test]
    fn test_query() {
        let mut deps = mock_dependencies();

        let info = mock_info("anyone", &[]);
        instantiate(deps.as_mut(), mock_env(), info, InstantiateMsg {}).unwrap();

        let sender1 = String::from("sender0001");
        let sender2 = String::from("sender0002");
        // Same balance for simplicity
        let balance = coins(1000, "tokens");

        // Create a couple swaps (same hash for simplicity)
        let info = mock_info(&sender1, &balance);
        let create1 = CreateMsg {
            id: "swap0001".to_string(),
            hash: custom_hash(1),
            recipient: "rcpt0001".into(),
            expires: Expiration::AtHeight(123456),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Create(create1.clone()),
        )
        .unwrap();

        let info = mock_info(&sender2, &balance);
        let create2 = CreateMsg {
            id: "swap0002".to_string(),
            hash: custom_hash(2),
            recipient: "rcpt0002".into(),
            expires: Expiration::AtTime(Timestamp::from_seconds(2_000_000_000)),
        };
        execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Create(create2.clone()),
        )
        .unwrap();

        // Get the list of ids
        let query_msg = QueryMsg::List {
            start_after: None,
            limit: None,
        };
        let ids: ListResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(2, ids.swaps.len());
        assert_eq!(vec!["swap0001", "swap0002"], ids.swaps);

        // Get the details for the first swap id
        let query_msg = QueryMsg::Details {
            id: ids.swaps[0].clone(),
        };
        let res: DetailsResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(
            res,
            DetailsResponse {
                id: create1.id,
                hash: create1.hash,
                recipient: create1.recipient,
                source: sender1,
                expires: create1.expires,
                balance: BalanceHuman::Native(balance.clone()),
            }
        );

        // Get the details for the second swap id
        let query_msg = QueryMsg::Details {
            id: ids.swaps[1].clone(),
        };
        let res: DetailsResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(
            res,
            DetailsResponse {
                id: create2.id,
                hash: create2.hash,
                recipient: create2.recipient,
                source: sender2,
                expires: create2.expires,
                balance: BalanceHuman::Native(balance),
            }
        );
    }

    #[test]
    fn test_native_cw20_swap() {
        let mut deps = mock_dependencies();

        // Create the contract
        let info = mock_info("anyone", &[]);
        let res = instantiate(deps.as_mut(), mock_env(), info, InstantiateMsg {}).unwrap();
        assert_eq!(0, res.messages.len());

        // Native side (offer)
        let native_sender = String::from("A_on_X");
        let native_rcpt = String::from("B_on_X");
        let native_coins = coins(1000, "tokens_native");

        // Create the Native swap offer
        let native_swap_id = "native_swap".to_string();
        let create = CreateMsg {
            id: native_swap_id.clone(),
            hash: real_hash(),
            recipient: native_rcpt.clone(),
            expires: Expiration::AtHeight(123456),
        };
        let info = mock_info(&native_sender, &native_coins);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Create(create)).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // Cw20 side (counter offer (1:1000))
        let cw20_sender = String::from("B_on_Y");
        let cw20_rcpt = String::from("A_on_Y");
        let cw20_coin = Cw20Coin {
            address: String::from("my_cw20_token"),
            amount: Uint128::new(1),
        };

        // Create the Cw20 side swap counter offer
        let cw20_swap_id = "cw20_swap".to_string();
        let create = CreateMsg {
            id: cw20_swap_id.clone(),
            hash: real_hash(),
            recipient: cw20_rcpt.clone(),
            expires: Expiration::AtHeight(123000),
        };
        let receive = Cw20ReceiveMsg {
            sender: cw20_sender,
            amount: cw20_coin.amount,
            msg: to_binary(&ExecuteMsg::Create(create)).unwrap(),
        };
        let token_contract = cw20_coin.address;
        let info = mock_info(&token_contract, &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Receive(receive),
        )
        .unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(("action", "create"), res.attributes[0]);

        // Somebody (typically, A) releases the swap side on the Cw20 (Y) blockchain,
        // using her knowledge of the preimage
        let info = mock_info("somebody", &[]);
        let res = execute(
            deps.as_mut(),
            mock_env(),
            info,
            ExecuteMsg::Release {
                id: cw20_swap_id.clone(),
                preimage: preimage(),
            },
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "release"), res.attributes[0]);
        assert_eq!(("id", cw20_swap_id), res.attributes[1]);

        // Verify the resulting Cw20 transfer message
        let send_msg = Cw20ExecuteMsg::Transfer {
            recipient: cw20_rcpt,
            amount: cw20_coin.amount,
        };
        assert_eq!(
            res.messages[0],
            SubMsg::new(WasmMsg::Execute {
                contract_addr: token_contract,
                msg: to_binary(&send_msg).unwrap(),
                funds: vec![],
            })
        );

        // Now somebody (typically, B) releases the original offer on the Native (X) blockchain,
        // using the (now public) preimage
        let info = mock_info("other_somebody", &[]);

        // First, let's obtain the preimage from the logs of the release() transaction on Y
        let preimage_attr = &res.attributes[2];
        assert_eq!("preimage", preimage_attr.key);
        let preimage = preimage_attr.value.clone();

        let release = ExecuteMsg::Release {
            id: native_swap_id.clone(),
            preimage,
        };
        let res = execute(deps.as_mut(), mock_env(), info, release).unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(("action", "release"), res.attributes[0]);
        assert_eq!(("id", native_swap_id), res.attributes[1]);

        // Verify the resulting Native send message
        assert_eq!(
            res.messages[0],
            SubMsg::new(BankMsg::Send {
                to_address: native_rcpt,
                amount: native_coins,
            })
        );
    }
}
