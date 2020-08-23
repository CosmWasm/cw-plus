use cosmwasm_std::{
    log, to_binary, Api, BankMsg, Binary, Coin, CosmosMsg, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, Querier, StdError, StdResult, Storage,
};
use sha2::{Digest, Sha256};

use cw2::{set_contract_version, ContractVersion};

use crate::msg::{
    is_valid_name, CreateMsg, DetailsResponse, HandleMsg, InitMsg, ListResponse, QueryMsg,
};
use crate::state::{all_swap_ids, atomic_swaps, atomic_swaps_read, AtomicSwap};

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:atomic-swap";
const CONTRACT_VERSION: &str = "v0.1.0";

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    _env: Env,
    _msg: InitMsg,
) -> StdResult<InitResponse> {
    let version = ContractVersion {
        contract: CONTRACT_NAME.to_string(),
        version: CONTRACT_VERSION.to_string(),
    };
    set_contract_version(&mut deps.storage, &version)?;
    // No setup
    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    match msg {
        HandleMsg::Create(msg) => try_create(deps, env, msg),
        HandleMsg::Release { id, preimage } => try_release(deps, env, id, preimage),
        HandleMsg::Refund { id } => try_refund(deps, env, id),
    }
}

pub fn try_create<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: CreateMsg,
) -> StdResult<HandleResponse> {
    if !is_valid_name(&msg.id) {
        return Err(StdError::generic_err("Invalid atomic swap id"));
    }

    if env.message.sent_funds.is_empty() {
        return Err(StdError::generic_err(
            "Send some coins to create an atomic swap",
        ));
    }

    // Ensure this is 32 bytes hex-encoded
    let _ = parse_hex_32(&msg.hash)?;

    let recipient_raw = deps.api.canonical_address(&msg.recipient)?;

    let swap = AtomicSwap {
        hash: msg.hash.clone(),
        recipient: recipient_raw,
        source: deps.api.canonical_address(&env.message.sender)?,
        end_height: msg.end_height,
        end_time: msg.end_time,
        balance: env.message.sent_funds.clone(),
    };

    if swap.is_expired(&env) {
        return Err(StdError::generic_err("Expired atomic swap"));
    }

    // Try to store it, fail if the id already exists (unmodifiable swaps)
    atomic_swaps(&mut deps.storage).update(msg.id.as_bytes(), |existing| match existing {
        None => Ok(swap),
        Some(_) => Err(StdError::generic_err("Atomic swap already exists")),
    })?;

    let mut res = HandleResponse::default();
    res.log = vec![
        log("action", "create"),
        log("id", msg.id),
        log("hash", msg.hash),
        log("recipient", msg.recipient),
    ];
    Ok(res)
}

pub fn try_release<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
    preimage: String,
) -> StdResult<HandleResponse> {
    let swap = atomic_swaps_read(&deps.storage).load(id.as_bytes())?;
    if swap.is_expired(&env) {
        return Err(StdError::generic_err("Atomic swap expired"));
    }

    let hash = Sha256::digest(&parse_hex_32(&preimage)?);
    let expected = parse_hex_32(&swap.hash)?;
    if hash.as_slice() != expected.as_slice() {
        return Err(StdError::generic_err("Invalid preimage"));
    }

    let rcpt = deps.api.human_address(&swap.recipient)?;

    // We delete the swap
    atomic_swaps(&mut deps.storage).remove(id.as_bytes());

    // Send all tokens out
    let msgs = send_native_tokens(&env.contract.address, &rcpt, swap.balance);
    Ok(HandleResponse {
        messages: msgs,
        log: vec![
            log("action", "release"),
            log("id", id),
            log("preimage", preimage),
            log("to", rcpt),
        ],
        data: None,
    })
}

pub fn try_refund<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    id: String,
) -> StdResult<HandleResponse> {
    let swap = atomic_swaps_read(&deps.storage).load(id.as_bytes())?;
    // Anyone can try to refund, as long as the contract is expired
    if !swap.is_expired(&env) {
        return Err(StdError::generic_err("Atomic swap not yet expired"));
    }

    let rcpt = deps.api.human_address(&swap.source)?;

    // We delete the swap
    atomic_swaps(&mut deps.storage).remove(id.as_bytes());

    let msgs = send_native_tokens(&env.contract.address, &rcpt, env.message.sent_funds);
    Ok(HandleResponse {
        messages: msgs,
        log: vec![log("action", "refund"), log("id", id), log("to", rcpt)],
        data: None,
    })
}

fn parse_hex_32(data: &str) -> StdResult<Vec<u8>> {
    match hex::decode(data) {
        Ok(bin) => {
            if bin.len() == 32 {
                Ok(bin)
            } else {
                Err(StdError::generic_err("Hash must be 64 characters"))
            }
        }
        Err(e) => Err(StdError::generic_err(format!(
            "Error parsing hash: {}",
            e.to_string()
        ))),
    }
}

fn send_native_tokens(from: &HumanAddr, to: &HumanAddr, amount: Vec<Coin>) -> Vec<CosmosMsg> {
    if amount.is_empty() {
        vec![]
    } else {
        vec![BankMsg::Send {
            from_address: from.into(),
            to_address: to.into(),
            amount,
        }
        .into()]
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::List {} => to_binary(&query_list(deps)?),
        QueryMsg::Details { id } => to_binary(&query_details(deps, id)?),
    }
}

fn query_details<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    id: String,
) -> StdResult<DetailsResponse> {
    let swap = atomic_swaps_read(&deps.storage).load(id.as_bytes())?;

    let details = DetailsResponse {
        id,
        hash: swap.hash,
        recipient: deps.api.human_address(&swap.recipient)?,
        source: deps.api.human_address(&swap.source)?,
        end_height: swap.end_height,
        end_time: swap.end_time,
        balance: swap.balance,
    };
    Ok(details)
}

fn query_list<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> StdResult<ListResponse> {
    Ok(ListResponse {
        swaps: all_swap_ids(&deps.storage)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, CosmosMsg, StdError};

    const CANONICAL_LENGTH: usize = 20;

    fn preimage() -> String {
        hex::encode(b"This is a string, 32 bytes long.")
    }
    fn real_hash() -> String {
        hex::encode(&Sha256::digest(&hex::decode(preimage()).unwrap()))
    }

    fn mock_env_height<U: Into<HumanAddr>>(sender: U, sent: &[Coin], height: u64) -> Env {
        let mut env = mock_env(sender, sent);
        env.block.height = height;
        env
    }

    #[test]
    fn test_init() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // Init an empty contract
        let init_msg = InitMsg {};
        let env = mock_env("anyone", &[]);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());
    }

    #[test]
    fn test_create() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        let env = mock_env("anyone", &[]);
        init(&mut deps, env, InitMsg {}).unwrap();

        let sender = HumanAddr::from("sender0001");
        let balance = coins(100, "tokens");

        // Cannot create, invalid ids
        let env = mock_env(&sender, &balance);
        for id in vec!["sh", "atomic_swap_id_too_long"] {
            let create = CreateMsg {
                id: id.to_string(),
                hash: real_hash(),
                recipient: HumanAddr::from("rcpt0001"),
                end_time: 0,
                end_height: 123456,
            };
            let res = handle(&mut deps, env.clone(), HandleMsg::Create(create.clone()));
            match res {
                Ok(_) => panic!("expected error"),
                Err(StdError::GenericErr { msg, .. }) => {
                    assert_eq!(msg, "Invalid atomic swap id".to_string())
                }
                Err(e) => panic!("unexpected error: {:?}", e),
            }
        }

        // Cannot create, no funds
        let env = mock_env(&sender, &vec![]);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            end_time: 0,
            end_height: 123456,
        };
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone()));
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Send some coins to create an atomic swap".to_string())
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Cannot create, expired
        let env = mock_env(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            end_height: 0,
            end_time: 1,
        };
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone()));
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Expired atomic swap".to_string())
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Cannot create, invalid hash
        let env = mock_env(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: "bu115h17".to_string(),
            recipient: "rcpt0001".into(),
            end_time: 0,
            end_height: 123456,
        };
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone()));
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(
                msg,
                "Error parsing hash: Invalid character \'u\' at position 1".to_string()
            ),
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Can create, all valid
        let env = mock_env(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            end_time: 0,
            end_height: 123456,
        };
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // Cannot re-create (modify), already existing
        let new_balance = coins(1, "tokens");
        let env = mock_env(&sender, &new_balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            end_time: 0,
            end_height: 123456,
        };
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone()));
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Atomic swap already exists".to_string())
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_release() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        let env = mock_env("anyone", &[]);
        init(&mut deps, env, InitMsg {}).unwrap();

        let sender = HumanAddr::from("sender0001");
        let balance = coins(1000, "tokens");

        let env = mock_env(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            end_time: 0,
            end_height: 123456,
        };
        handle(&mut deps, env.clone(), HandleMsg::Create(create.clone())).unwrap();

        // Cannot release, wrong id
        let release = HandleMsg::Release {
            id: "swap0002".to_string(),
            preimage: preimage(),
        };
        let res = handle(&mut deps, env.clone(), release);
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::NotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Cannot release, invalid hash
        let release = HandleMsg::Release {
            id: "swap0001".to_string(),
            preimage: "bu115h17".to_string(),
        };
        let res = handle(&mut deps, env.clone(), release);
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => assert_eq!(
                msg,
                "Error parsing hash: Invalid character \'u\' at position 1".to_string()
            ),
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Cannot release, wrong hash
        let release = HandleMsg::Release {
            id: "swap0001".to_string(),
            preimage: hex::encode(b"This is 32 bytes, but incorrect."),
        };
        let res = handle(&mut deps, env.clone(), release);
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Invalid preimage".to_string())
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Cannot release, expired
        let env = mock_env_height(&sender, &balance, 123457);
        let release = HandleMsg::Release {
            id: "swap0001".to_string(),
            preimage: preimage(),
        };
        let res = handle(&mut deps, env.clone(), release);
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Atomic swap expired".to_string())
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Can release, valid id, valid hash, and not expired
        let env = mock_env("somebody", &balance);
        let release = HandleMsg::Release {
            id: "swap0001".to_string(),
            preimage: preimage(),
        };
        let res = handle(&mut deps, env.clone(), release.clone()).unwrap();
        assert_eq!(log("action", "release"), res.log[0]);
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: create.recipient,
                amount: balance,
            })
        );

        // Cannot release again
        let res = handle(&mut deps, env.clone(), release);
        match res.unwrap_err() {
            StdError::NotFound { .. } => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }

    #[test]
    fn test_refund() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        let env = mock_env("anyone", &[]);
        init(&mut deps, env, InitMsg {}).unwrap();

        let sender = HumanAddr::from("sender0001");
        let balance = coins(1000, "tokens");

        let env = mock_env(&sender, &balance);
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: "rcpt0001".into(),
            end_time: 0,
            end_height: 123456,
        };
        handle(&mut deps, env.clone(), HandleMsg::Create(create.clone())).unwrap();

        // Cannot refund, wrong id
        let refund = HandleMsg::Refund {
            id: "swap0002".to_string(),
        };
        let res = handle(&mut deps, env.clone(), refund);
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::NotFound { .. }) => {}
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Cannot refund, not expired yet
        let refund = HandleMsg::Refund {
            id: "swap0001".to_string(),
        };
        let res = handle(&mut deps, env.clone(), refund);
        match res {
            Ok(_) => panic!("expected error"),
            Err(StdError::GenericErr { msg, .. }) => {
                assert_eq!(msg, "Atomic swap not yet expired".to_string())
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // Can refund, already expired
        let env = mock_env_height("anybody", &balance, 123457);
        let refund = HandleMsg::Refund {
            id: "swap0001".to_string(),
        };
        let res = handle(&mut deps, env.clone(), refund.clone()).unwrap();
        assert_eq!(log("action", "refund"), res.log[0]);
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: sender,
                amount: balance,
            })
        );

        // Cannot refund again
        let res = handle(&mut deps, env.clone(), refund);
        match res.unwrap_err() {
            StdError::NotFound { .. } => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }

    #[test]
    fn happy_path() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        // Init an empty contract
        let init_msg = InitMsg {};
        let env = mock_env("anyone", &[]);
        let res = init(&mut deps, env, init_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Create a swap
        let create = CreateMsg {
            id: "swap0001".to_string(),
            hash: real_hash(),
            recipient: HumanAddr::from("rcpt0001"),
            end_time: 0,
            end_height: 123456,
        };
        let sender = HumanAddr::from("sender0001");
        let balance = coins(100, "tokens");
        let env = mock_env(&sender, &balance);
        let res = handle(&mut deps, env, HandleMsg::Create(create.clone())).unwrap();
        assert_eq!(0, res.messages.len());
        assert_eq!(log("action", "create"), res.log[0]);

        // Release it
        let id = create.id.clone();
        let env = mock_env(&create.recipient, &[]);
        let res = handle(
            &mut deps,
            env.clone(),
            HandleMsg::Release {
                id,
                preimage: preimage(),
            },
        )
        .unwrap();
        assert_eq!(1, res.messages.len());
        assert_eq!(log("action", "release"), res.log[0]);
        assert_eq!(
            res.messages[0],
            CosmosMsg::Bank(BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: create.recipient,
                amount: balance,
            })
        );

        // Second attempt fails (not found)
        let id = create.id.clone();
        let res = handle(
            &mut deps,
            env,
            HandleMsg::Release {
                id,
                preimage: preimage(),
            },
        );
        match res.unwrap_err() {
            StdError::NotFound { .. } => {}
            e => panic!("Expected NotFound, got {}", e),
        }
    }
}
