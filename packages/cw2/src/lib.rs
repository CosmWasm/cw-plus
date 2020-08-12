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

/// get_contract_info can be use in migrate to read the previous version of this contract
pub fn get_contract_info<S: ReadonlyStorage>(storage: &S) -> StdResult<ContractInfo> {
    ReadonlySingleton::new(storage, PREFIX_INFO).load()
}

/// set_contract_info should be used in init to store the original version, and after a successful
/// migrate to update it
pub fn set_contract_info<S: Storage>(storage: &mut S, info: &ContractInfo) -> StdResult<()> {
    Singleton::new(storage, PREFIX_INFO).save(info)
}

/// This will make a raw_query to another contract to determine the current version it
/// claims to be. This should not be trusted, but could be used as a quick filter
/// if the other contract exists and claims to be a cw20-base contract for example.
/// (Note: you usually want to require *interfaces* not *implementations* of the
/// contracts you compose with, so be careful of overuse)
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
    use crate::{get_contract_info, set_contract_info, ContractInfo, ContractVersion};
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn get_and_set_work() {
        let mut store = MockStorage::new();

        // error if not set
        assert!(get_contract_info(&store).is_err());

        // set and get
        let info = ContractInfo {
            versions: vec![ContractVersion {
                contract: "crate:cw20-base".to_string(),
                version: "v0.1.0".to_string(),
            }],
        };
        set_contract_info(&mut store, &info).unwrap();
        let loaded = get_contract_info(&store).unwrap();
        assert_eq!(info, loaded);
    }
}
