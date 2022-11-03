use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Empty, Querier, QuerierWrapper, QueryRequest, StdResult, Storage, WasmQuery};
use cw_storage_plus::Item;

pub const CONTRACT: Item<ContractVersion> = Item::new("contract_info");

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ContractVersion {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.7.0", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,
    /// supported_interface is an optional parameter returning a vector of string represents interfaces 
    /// that the contract support The string value is the interface crate names in Rust crate Registry. 
    /// This parameter is inspired by the EIP-165 from Ethereum.
    /// Each string value should follow a common standard such as <Registry Domain>:<Crate Name>  
    /// e.g ["crates.io:cw721","crates.io:cw2"]
    /// NOTE: this is just a hint for the caller to adapt on how to interact with this contract.
    /// There is no guarantee that the contract actually implement these interfaces.
    pub supported_interface: Option<Vec<String>>,
}

/// get_contract_version can be use in migrate to read the previous version of this contract
pub fn get_contract_version(store: &dyn Storage) -> StdResult<ContractVersion> {
    CONTRACT.load(store)
}

/// set_contract_version should be used in instantiate to store the original version, and after a successful
/// migrate to update it
pub fn set_contract_version<T: Into<String>, U: Into<String>>(
    store: &mut dyn Storage,
    name: T,
    version: U,
    supported_interface: Option<Vec<String>>,
) -> StdResult<()> {
    let val = ContractVersion {
        contract: name.into(),
        version: version.into(),
        supported_interface: supported_interface.into(),
    };
    CONTRACT.save(store, &val)
}

/// This will make a raw_query to another contract to determine the current version it
/// claims to be. This should not be trusted, but could be used as a quick filter
/// if the other contract exists and claims to be a cw20-base contract for example.
/// (Note: you usually want to require *interfaces* not *implementations* of the
/// contracts you compose with, so be careful of overuse)
pub fn query_contract_info<Q: Querier, T: Into<String>>(
    querier: &Q,
    contract_addr: T,
) -> StdResult<ContractVersion> {
    let req = QueryRequest::Wasm(WasmQuery::Raw {
        contract_addr: contract_addr.into(),
        key: CONTRACT.as_slice().into(),
    });
    QuerierWrapper::<Empty>::new(querier).query(&req)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;
    use std::vec::Vec;

    #[test]
    fn get_and_set_work() {
        let mut store = MockStorage::new();

        // error if not set
        assert!(get_contract_version(&store).is_err());

        // set and get
        let contract_name = "crate:cw20-base";
        let contract_version = "0.2.0";
        set_contract_version(&mut store, contract_name, contract_version,None).unwrap();

        let loaded = get_contract_version(&store).unwrap();
        let expected = ContractVersion {
            contract: contract_name.to_string(),
            version: contract_version.to_string(),
            supported_interface: None,
        };
        assert_eq!(expected, loaded);

        // set and get with supported_interface
        let contract_name = "crate:cw20-base";
        let contract_version = "0.2.0";
        let supported_interface = Some(Vec::from(["crates.io:cw2".to_string(),"crates.io:cw721".to_string()]));
        set_contract_version(&mut store, contract_name, contract_version,supported_interface.clone()).unwrap();

        let loaded = get_contract_version(&store).unwrap();
        let expected = ContractVersion {
            contract: contract_name.to_string(),
            version: contract_version.to_string(),
            supported_interface: supported_interface.clone(),
        };
        assert_eq!(expected, loaded);
    }
}
