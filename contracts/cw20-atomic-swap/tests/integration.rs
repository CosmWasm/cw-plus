//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests as follows:
//! 1. Copy them over verbatim
//! 2. Then change
//!      let mut deps = mock_dependencies(20, &[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, wherever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see query(&deps, ...) you must replace it with query(&mut deps, ...)

use cosmwasm_std::{coins, from_binary, BankMsg, Coin, CosmosMsg, Env, StdError};
use cosmwasm_std::{log, HandleResponse, HandleResult, HumanAddr, InitResponse};
use cosmwasm_vm::testing::{handle, init, mock_env, mock_instance, query};
use sha2::{Digest, Sha256};

use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cw20_atomic_swap::msg::{
    CreateMsg, DetailsResponse, HandleMsg, InitMsg, ListResponse, QueryMsg,
};

// This line will test the output of cargo wasm
static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/cw20_atomic_swap.wasm");
// Uncomment this line instead to test productivized build from rust-optimizer
// static WASM: &[u8] = include_bytes!("../contract.wasm");

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

fn mock_env_height<U: Into<HumanAddr>>(sender: U, sent: &[Coin], height: u64) -> Env {
    let mut env = mock_env(sender, sent);
    env.block.height = height;
    env
}

#[test]
fn test_init() {
    let mut deps = mock_instance(WASM, &[]);

    // Init an empty contract
    let init_msg = InitMsg {};
    let env = mock_env("anyone", &[]);
    let res: InitResponse = init(&mut deps, env, init_msg).unwrap();
    assert_eq!(0, res.messages.len());
}

#[test]
fn test_create() {
    let mut deps = mock_instance(WASM, &[]);

    let env = mock_env("anyone", &[]);
    let _: InitResponse = init(&mut deps, env, InitMsg {}).unwrap();

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
        let res: HandleResult = handle(&mut deps, env.clone(), HandleMsg::Create(create.clone()));
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
    let res: HandleResult = handle(&mut deps, env, HandleMsg::Create(create.clone()));
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
    let res: HandleResult = handle(&mut deps, env, HandleMsg::Create(create.clone()));
    match res {
        Ok(_) => panic!("expected error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Expired atomic swap".to_string()),
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
    let res: HandleResult = handle(&mut deps, env, HandleMsg::Create(create.clone()));
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
    let res: HandleResponse = handle(&mut deps, env, HandleMsg::Create(create.clone())).unwrap();
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
    let res: HandleResult = handle(&mut deps, env, HandleMsg::Create(create.clone()));
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
    let mut deps = mock_instance(WASM, &[]);

    let env = mock_env("anyone", &[]);
    let _: InitResponse = init(&mut deps, env, InitMsg {}).unwrap();

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
    let _: HandleResponse =
        handle(&mut deps, env.clone(), HandleMsg::Create(create.clone())).unwrap();

    // Anyone can attempt release
    let env = mock_env("somebody", &[]);

    // Cannot release, wrong id
    let release = HandleMsg::Release {
        id: "swap0002".to_string(),
        preimage: preimage(),
    };
    let res: HandleResult = handle(&mut deps, env.clone(), release);
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
    let res: HandleResult = handle(&mut deps, env.clone(), release);
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
    let res: HandleResult = handle(&mut deps, env.clone(), release);
    match res {
        Ok(_) => panic!("expected error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Invalid preimage".to_string()),
        Err(e) => panic!("unexpected error: {:?}", e),
    }

    // Cannot release, expired
    let env = mock_env_height(&sender, &balance, 123457);
    let release = HandleMsg::Release {
        id: "swap0001".to_string(),
        preimage: preimage(),
    };
    let res: HandleResult = handle(&mut deps, env.clone(), release);
    match res {
        Ok(_) => panic!("expected error"),
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Atomic swap expired".to_string()),
        Err(e) => panic!("unexpected error: {:?}", e),
    }

    // Can release, valid id, valid hash, and not expired
    let env = mock_env("somebody", &[]);
    let release = HandleMsg::Release {
        id: "swap0001".to_string(),
        preimage: preimage(),
    };
    let res: HandleResponse = handle(&mut deps, env.clone(), release.clone()).unwrap();
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
    let res: HandleResult = handle(&mut deps, env.clone(), release);
    match res.unwrap_err() {
        StdError::NotFound { .. } => {}
        e => panic!("Expected NotFound, got {}", e),
    }
}

#[test]
fn test_refund() {
    let mut deps = mock_instance(WASM, &[]);

    let env = mock_env("anyone", &[]);
    let _: InitResponse = init(&mut deps, env, InitMsg {}).unwrap();

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
    let _: HandleResponse =
        handle(&mut deps, env.clone(), HandleMsg::Create(create.clone())).unwrap();

    // Anyone can attempt refund
    let env = mock_env("somebody", &[]);

    // Cannot refund, wrong id
    let refund = HandleMsg::Refund {
        id: "swap0002".to_string(),
    };
    let res: HandleResult = handle(&mut deps, env.clone(), refund);
    match res {
        Ok(_) => panic!("expected error"),
        Err(StdError::NotFound { .. }) => {}
        Err(e) => panic!("unexpected error: {:?}", e),
    }

    // Cannot refund, not expired yet
    let refund = HandleMsg::Refund {
        id: "swap0001".to_string(),
    };
    let res: HandleResult = handle(&mut deps, env.clone(), refund);
    match res {
        Ok(_) => panic!("expected error"),
        Err(StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "Atomic swap not yet expired".to_string())
        }
        Err(e) => panic!("unexpected error: {:?}", e),
    }

    // Anyone can refund, if already expired
    let env = mock_env_height("somebody", &[], 123457);
    let refund = HandleMsg::Refund {
        id: "swap0001".to_string(),
    };
    let res: HandleResponse = handle(&mut deps, env.clone(), refund.clone()).unwrap();
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
    let res: HandleResult = handle(&mut deps, env.clone(), refund);
    match res.unwrap_err() {
        StdError::NotFound { .. } => {}
        e => panic!("Expected NotFound, got {}", e),
    }
}

#[test]
fn test_query() {
    let mut deps = mock_instance(WASM, &[]);

    let env = mock_env("anyone", &[]);
    let _: InitResponse = init(&mut deps, env, InitMsg {}).unwrap();

    let sender1 = HumanAddr::from("sender0001");
    let sender2 = HumanAddr::from("sender0002");
    // Same balance for simplicity
    let balance = coins(1000, "tokens");

    // Create a couple swaps (same hash for simplicity)
    let env = mock_env(&sender1, &balance);
    let create1 = CreateMsg {
        id: "swap0001".to_string(),
        hash: custom_hash(1),
        recipient: "rcpt0001".into(),
        end_time: 0,
        end_height: 123456,
    };
    let _: HandleResponse =
        handle(&mut deps, env.clone(), HandleMsg::Create(create1.clone())).unwrap();

    let env = mock_env(&sender2, &balance);
    let create2 = CreateMsg {
        id: "swap0002".to_string(),
        hash: custom_hash(2),
        recipient: "rcpt0002".into(),
        end_time: 2_000_000_000,
        end_height: 0,
    };
    let _: HandleResponse =
        handle(&mut deps, env.clone(), HandleMsg::Create(create2.clone())).unwrap();

    // Get the list of ids
    let query_msg = QueryMsg::List {};
    let ids: ListResponse = from_binary(&query(&mut deps, query_msg).unwrap()).unwrap();
    assert_eq!(2, ids.swaps.len());
    assert_eq!(vec!["swap0001", "swap0002"], ids.swaps);

    // Get the details for the first swap id
    let query_msg = QueryMsg::Details {
        id: ids.swaps[0].clone(),
    };
    let res: DetailsResponse = from_binary(&query(&mut deps, query_msg).unwrap()).unwrap();
    assert_eq!(
        res,
        DetailsResponse {
            id: create1.id,
            hash: create1.hash,
            recipient: create1.recipient,
            source: sender1,
            end_height: create1.end_height,
            end_time: create1.end_time,
            balance: balance.clone()
        }
    );

    // Get the details for the second swap id
    let query_msg = QueryMsg::Details {
        id: ids.swaps[1].clone(),
    };
    let res: DetailsResponse = from_binary(&query(&mut deps, query_msg).unwrap()).unwrap();
    assert_eq!(
        res,
        DetailsResponse {
            id: create2.id,
            hash: create2.hash,
            recipient: create2.recipient,
            source: sender2,
            end_height: create2.end_height,
            end_time: create2.end_time,
            balance
        }
    );
}
