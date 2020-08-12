use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    HumanAddr, Querier, QueryRequest, ReadonlyStorage, StdResult, Storage, WasmQuery,
};
use cosmwasm_storage::{to_length_prefixed, ReadonlySingleton, Singleton};

pub const PREFIX_INFO: &[u8] = b"contract_info";

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractVersion {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.6.2", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractInfo {
    versions: Vec<ContractVersion>,
}

pub fn get_contract_info<S: ReadonlyStorage>(storage: &S) -> StdResult<ContractInfo> {
    ReadonlySingleton::new(storage, PREFIX_INFO).load()
}

pub fn set_contract_info<S: Storage>(storage: &mut S, info: &ContractInfo) -> StdResult<()> {
    Singleton::new(storage, PREFIX_INFO).save(info)
}

pub fn query_contract_info<Q: Querier, T: Into<HumanAddr>>(
    querier: &Q,
    contract_addr: T,
) -> StdResult<ContractInfo> {
    let req = QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: contract_addr.into(),
        key: to_length_prefixed(PREFIX_INFO).into(),
    });
    querier.query(&req)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
