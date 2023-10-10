use cw_orch::{
    interface,
    prelude::*,
};

use cw4_group::contract;
use cw4_group::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct Cw4Group;

impl<Chain: CwEnv> Uploadable for Cw4Group<Chain> {
    // Return the path to the wasm file
    fn wasm(&self) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw4_group.wasm").unwrap()
    }
    // Return a CosmWasm contract wrapper
    fn wrapper(&self) -> Box<dyn MockContract<Empty>> {
        Box::new(
            ContractWrapper::new_with_empty(
                contract::execute,
                contract::instantiate,
                contract::query,
            )
        )
    }
}