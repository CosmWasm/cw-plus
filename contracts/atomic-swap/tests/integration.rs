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

use cosmwasm_std::InitResponse;
use cosmwasm_vm::testing::{init, mock_env, mock_instance};
//use cosmwasm_vm::{from_slice, Api, Storage};
use atomic_swap::msg::InitMsg;

// This line will test the output of cargo wasm
static WASM: &[u8] =
    include_bytes!("../../../target/wasm32-unknown-unknown/release/atomic_swap.wasm");
// Uncomment this line instead to test productivized build from rust-optimizer
// static WASM: &[u8] = include_bytes!("../contract.wasm");

#[test]
fn test_init() {
    let mut deps = mock_instance(WASM, &[]);

    // Init an empty contract
    let init_msg = InitMsg {};
    let env = mock_env("anyone", &[]);
    let res: InitResponse = init(&mut deps, env, init_msg).unwrap();
    assert_eq!(0, res.messages.len());
}
