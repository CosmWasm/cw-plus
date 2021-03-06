//! This integration test tries to run and call the generated wasm.
//! It depends on a Wasm build being available, which you can create with `cargo wasm`.
//! Then running `cargo integration-test` will validate we can properly call into that generated Wasm.
//!
//! You can easily convert unit tests to integration tests.
//! 1. First copy them over verbatim,
//! 2. Then change
//!      let mut deps = mock_dependencies(20, &[]);
//!    to
//!      let mut deps = mock_instance(WASM, &[]);
//! 3. If you access raw storage, where ever you see something like:
//!      deps.storage.get(CONFIG_KEY).expect("no data stored");
//!    replace it with:
//!      deps.with_storage(|store| {
//!          let data = store.get(CONFIG_KEY).expect("no data stored");
//!          //...
//!      });
//! 4. Anywhere you see init/execute(deps.as_mut(), ...) you must replace it with init/execute(&mut deps, ...)
//! 5. Anywhere you see query(deps.as_ref(), ...) you must replace it with query(&mut deps, ...)
//! (Use cosmwasm_vm::testing::{init, execute, query}, instead of the contract variants).

use cosmwasm_std::Response;
use cosmwasm_vm::testing::{
    init, mock_env, mock_info, mock_instance, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_vm::Instance;

use cw20_bonding::msg::{CurveType, InitMsg};

// Output of cargo wasm
static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/cw20_bonding.wasm");

const CREATOR: &str = "creator";

fn setup() -> Instance<MockApi, MockStorage, MockQuerier> {
    let mut deps = mock_instance(WASM, &[]);
    let msg = InitMsg {
        name: "works?".to_string(),
        symbol: "one".to_string(),
        decimals: 0,
        reserve_denom: "naught".to_string(),
        reserve_decimals: 0,
        curve_type: CurveType::Constant {
            value: Default::default(),
            scale: 0,
        },
    };
    let info = mock_info(CREATOR, &[]);
    let res: Response = init(&mut deps, mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    deps
}

#[test]
fn init_works() {
    setup();
}
