use cw_orch::{
    interface,
    prelude::*,
};

use cw3_flex_multisig::contract;
use cw3_flex_multisig::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

#[interface(InstantiateMsg, ExecuteMsg, QueryMsg, Empty)]
pub struct Cw3FlexMultisig;

impl<Chain: CwEnv> Uploadable for Cw3FlexMultisig<Chain> {
    // Return the path to the wasm file
    fn wasm(&self) -> WasmPath {
        artifacts_dir_from_workspace!()
            .find_wasm_path("cw3_flex_multisig.wasm").unwrap()
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