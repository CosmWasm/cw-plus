/*!
Most of the CW* specs are focused on the *public interfaces*
of the contract. The APIs used for `ExecuteMsg` or `QueryMsg`.
However, when we wish to migrate or inspect smart contract info,
we need some form of smart contract information embedded on state.

This is where CW2 comes in. It specifies a special Item to
be stored on disk by all contracts on `instantiate`.

`ContractInfo` must be stored under the `"contract_info"` key which translates
to `"636F6E74726163745F696E666F"` in hex format.
Since the state is well defined, we do not need to support any "smart queries".
We do provide a helper to construct a "raw query" to read the ContractInfo
of any CW2-compliant contract.

For more information on this specification, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw2/README.md).
*/

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Empty, Querier, QuerierWrapper, QueryRequest, StdResult, Storage, WasmQuery};
use cw_storage_plus::Item;

pub const CONTRACT: Item<ContractVersion> = Item::new("contract_info");

#[cw_serde]
pub struct ContractVersion {
    /// contract is the crate name of the implementing contract, eg. `crate:cw20-base`
    /// we will use other prefixes for other languages, and their standard global namespacing
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.7.0", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,
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
) -> StdResult<()> {
    let val = ContractVersion {
        contract: name.into(),
        version: version.into(),
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

    #[test]
    fn get_and_set_work() {
        let mut store = MockStorage::new();

        // error if not set
        assert!(get_contract_version(&store).is_err());

        // set and get
        let contract_name = "crate:cw20-base";
        let contract_version = "0.2.0";
        set_contract_version(&mut store, contract_name, contract_version).unwrap();

        let loaded = get_contract_version(&store).unwrap();
        let expected = ContractVersion {
            contract: contract_name.to_string(),
            version: contract_version.to_string(),
        };
        assert_eq!(expected, loaded);
    }
}
