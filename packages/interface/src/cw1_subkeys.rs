use cw_orch::{interface, prelude::*};

use cw1_subkeys::contract;
pub use cw1_subkeys::msg::{ExecuteMsg, QueryMsg};
pub use cw1_whitelist::msg::InstantiateMsg;

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct Cw1SubKeys;

impl<Chain: CwEnv> Uploadable for Cw1SubKeys<Chain> {
    // Return the path to the wasm file
    fn wasm(&self) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw1_subkeys.wasm")
            .unwrap()
    }
    // Return a CosmWasm contract wrapper
    fn wrapper(&self) -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(
                contract::execute,
                contract::instantiate,
                contract::query,
            )
            .with_migrate(contract::migrate),
        )
    }
}
