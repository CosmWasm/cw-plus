# CW22 Spec: Contract Info

The standard used to declare which interface contract implements. This standard is inspired by the EIP-165 from
Ethereum.

For more information on this specification, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw22/README.md).

### Data structures

**Required**

All CW22-compliant contracts must store the following data:

* key: `contract_supported_interface`
* data: Json-serialized `ContractSupportedInterface`

```rust
pub struct ContractSupportedInterface<'a> {
    /// supported_interface is the name of an interface that the contract supports. 
    /// This is inspired by the EIP-165 from Ethereum.
    /// Interface names should follow a common standard such as <Registry Domain>:<Crate Name> in Rust crate registry.
    /// e.g. "crates.io:cw2"
    /// NOTE: this is just a hint for the caller to adapt on how to interact with this contract.
    /// There is no guarantee that the contract actually implements the interface.
    pub supported_interface: Cow<'a, str>,
    /// Version on release tags of the interface package following [SemVer](https://semver.org/) guideline.
    /// e.g.  "0.16.0"
    pub version: Cow<'a, str>,
}
```

Below is an example used in cw20 contract, where we declare to implement cw20 interface with version 0.16.0 at
instantiate:

```rust
use cw22::{set_contract_supported_interface, ContractSupportedInterface};

pub fn instantiate(
  mut deps: DepsMut,
  _env: Env,
  _info: MessageInfo,
  msg: InstantiateMsg,
) -> Result<Response, ContractError> {
  ///...
  let supported_interface = ContractSupportedInterface {
    supported_interface: "crates.io:cw20".into(),
    version: "0.16.0".into(),
  };
  set_contract_supported_interface(deps.storage, &[supported_interface])?;
  ///...
}
```
