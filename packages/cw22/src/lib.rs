/*!
The standard used to declare which interface contract implements
This standard is inspired by the EIP-165 from Ethereum.

For more information on this specification, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw22/README.md).
*/

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Item;

pub const CONTRACT: Item<ContractSupportedInterface> = Item::new("contract_supported_interface");

#[cw_serde]
pub struct ContractSupportedInterface {
    /// supported_interface is an optional parameter returning a vector of string represents interfaces
    /// that the contract support The string value is the interface crate names in Rust crate Registry.
    /// This parameter is inspired by the EIP-165 from Ethereum.
    /// Each string value should follow a common standard such as <Registry Domain>:<Crate Name>
    /// e.g ["crates.io:cw22","crates.io:cw2"]
    /// NOTE: this is just a hint for the caller to adapt on how to interact with this contract.
    /// There is no guarantee that the contract actually implement these interfaces.
    pub supported_interface: Vec<String>,
}

/// get_contract_version can be use in migrate to read the previous version of this contract
pub fn get_contract_supported_interface(
    store: &dyn Storage,
) -> StdResult<ContractSupportedInterface> {
    CONTRACT.load(store)
}

/// set_contract_version should be used in instantiate to store the original version, and after a successful
/// migrate to update it
pub fn set_contract_supported_interface(
    store: &mut dyn Storage,
    supported_interface: Vec<String>,
) -> StdResult<()> {
    let val = ContractSupportedInterface {
        supported_interface,
    };
    CONTRACT.save(store, &val)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn get_and_set_work() {
        let mut store = MockStorage::new();

        // error if not set
        assert!(get_contract_supported_interface(&store).is_err());

        // set and get with supported_interface
        let supported_interface: Vec<String> =
            Vec::from(["crates.io:cw2".to_string(), "crates.io:cw22".to_string()]);

        let v_ref = &supported_interface;
        set_contract_supported_interface(&mut store, v_ref.clone()).unwrap();

        let loaded = get_contract_supported_interface(&store).unwrap();
        let expected = ContractSupportedInterface {
            supported_interface: v_ref.clone(),
        };
        assert_eq!(expected, loaded);
    }
}
