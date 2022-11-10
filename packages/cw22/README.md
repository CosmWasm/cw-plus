# CW22 Spec: Contract Info

The standard used to declare which interface contract implements
This standard is inspired by the EIP-165 from Ethereum.

For more information on this specification, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw22/README.md).

### Data structures

**Required**

All CW22-compliant contracts must store the following data:

* key: `contract_supported_interface`
* data: Json-serialized `ContractSupportedInterface`

```rust
pub struct ContractSupportedInterface {
    /// supported_interface is an optional parameter returning a vector of string represents interfaces
    /// that the contract support The string value is the interface crate names in Rust crate Registry.
    /// This parameter is inspired by the EIP-165 from Ethereum.
    /// Each string value should follow a common standard such as <Registry Domain>:<Crate Name>
    /// e.g ["crates.io:cw721","crates.io:cw2"]
    /// NOTE: this is just a hint for the caller to adapt on how to interact with this contract.
    /// There is no guarantee that the contract actually implement these interfaces.
    pub supported_interface: Vec<String>,
}
```

Thus, an serialized example may looks like:

```json
{
    "supported_interface": ["crates.io:cw721","crates.io:cw2"]
}
```
