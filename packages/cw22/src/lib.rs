/*!
CW22 defines a way for a contract to declare which interfaces do the contract implement
This standard is inspired by the EIP-165 from Ethereum. Originally it was proposed to
be merged into CW2: Contract Info, then it is splitted to a separated cargo to keep CW2
being backward compatible.

Each supported interface contains a string value pointing to the corresponding cargo package
and a specific release of the package. There is also a function to check whether the contract
support a specific version of an interface or not.

The version string for each interface follows Semantic Versioning standard. More info is in:
https://docs.rs/semver/latest/semver/
*/

mod query;

use cosmwasm_schema::cw_serde;
use cosmwasm_std::StdError;
use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::Map;
use query::VersionResponse;
use semver::{Version, VersionReq};

pub const SUPPORTED_INTERFACES: Map<String, String> = Map::new("supported_interfaces");

#[cw_serde]
pub struct ContractSupportedInterface {
    /// supported_interface is an optional parameter returning a vector of string represents interfaces
    /// that the contract support The string value is the interface crate names in Rust crate Registry.
    /// This parameter is inspired by the EIP-165 from Ethereum.
    /// Each string value should follow a common standard such as <Registry Domain>:<Crate Name>
    /// e.g "crates.io:cw2"
    /// NOTE: this is just a hint for the caller to adapt on how to interact with this contract.
    /// There is no guarantee that the contract actually implement these interfaces.
    pub supported_interface: String,
    /// semantic version on release tags of the interface package following SemVer guideline.
    /// e.g  "0.16.0"
    pub version: String,
}

/// set_contract_supported_interface should be used in instantiate to store the original version
/// of supported interfaces. It should also be used after every migration.
pub fn set_contract_supported_interface(
    store: &mut dyn Storage,
    mut supported_interfaces: Vec<ContractSupportedInterface>,
) -> Result<(), StdError> {
    while let Some(supported_interface) = supported_interfaces.pop() {
        let id = supported_interface.supported_interface;
        let version = supported_interface.version;
        SUPPORTED_INTERFACES.save(store, id, &version)?;
    }
    Ok(())
}

/// query_supported_interface_version show the version of an interface supported by the contract
pub fn query_supported_interface_version(
    store: &dyn Storage,
    interface: String,
) -> StdResult<ContractSupportedInterface> {
    let version = SUPPORTED_INTERFACES
        .may_load(store, interface.clone())?
        .unwrap_or_default();
    let res = ContractSupportedInterface {
        supported_interface: interface,
        version,
    };
    Ok(res)
}

/// query_supported_interface show if contract supports an interface with version following SemVer query
/// query example">=1.2.3, <1.8.0"
pub fn query_supported_interface(
    store: &dyn Storage,
    interface: String,
    query: String,
) -> StdResult<VersionResponse> {
    let req = VersionReq::parse(&query).unwrap();
    let supported_version_rs = SUPPORTED_INTERFACES
        .may_load(store, interface)?
        .unwrap_or_default();
    let supported_version = Version::parse(&supported_version_rs);
    match supported_version {
        Ok(ver) => Ok(VersionResponse {
            version_require: query,
            supported_version: supported_version_rs,
            result: req.matches(&ver),
        }),
        Err(_) => Ok(VersionResponse {
            version_require: query,
            supported_version: supported_version_rs,
            result: false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn get_and_set_work() {
        let mut store = MockStorage::new();

        let interface = "crates.io:cw2";
        let interface2 = "crates.io:cw22";
        let contract_interface = ContractSupportedInterface {
            supported_interface: String::from(interface),
            version: String::from("0.16.0"),
        };
        let contract_interface2 = ContractSupportedInterface {
            supported_interface: String::from(interface2),
            version: String::from("0.1.0"),
        };

        // set and get with supported_interface
        let supported_interface: Vec<ContractSupportedInterface> =
            Vec::from([contract_interface, contract_interface2]);

        set_contract_supported_interface(&mut store, supported_interface).unwrap();

        // get version of not supported interface
        let loaded =
            query_supported_interface_version(&store, "crates.io:cw721".to_string()).unwrap();
        let expected = ContractSupportedInterface {
            supported_interface: "crates.io:cw721".to_string(),
            version: "".to_string(),
        };
        assert_eq!(expected, loaded);

        // get version of supported interface
        let loaded =
            query_supported_interface_version(&store, "crates.io:cw2".to_string()).unwrap();
        let expected = ContractSupportedInterface {
            supported_interface: "crates.io:cw2".to_string(),
            version: "0.16.0".to_string(),
        };
        assert_eq!(expected, loaded);

        // check specified version of not supported interface
        let version_req = ">=0.1.0".to_string();
        let result =
            query_supported_interface(&store, "crates.io:cw721".to_string(), version_req.clone())
                .unwrap();
        let expected = VersionResponse {
            version_require: version_req,
            supported_version: "".to_string(),
            result: false,
        };
        assert_eq!(expected, result);

        // check specified version of supported interface
        let version_req = ">=1.2.3, <1.8.0".to_string();
        let result =
            query_supported_interface(&store, "crates.io:cw2".to_string(), version_req.clone())
                .unwrap();
        let expected = VersionResponse {
            version_require: version_req,
            supported_version: "0.16.0".to_string(),
            result: false,
        };
        assert_eq!(expected, result);

        let version_req = ">=0.1.0".to_string();
        let result =
            query_supported_interface(&store, "crates.io:cw2".to_string(), version_req.clone())
                .unwrap();
        let expected = VersionResponse {
            version_require: version_req,
            supported_version: "0.16.0".to_string(),
            result: true,
        };
        assert_eq!(expected, result);
    }
}
