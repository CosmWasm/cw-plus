# CW2 Spec: Contract Info

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

You can query using:
```shell
wasmd query wasm contract-state raw [contract_addr] 636F6E74726163745F696E666F --node $RPC
```

When the `migrate` function is called, then the new contract
can read that data andsee if this is an expected contract we can 
migrate from. And also contain extra version information if we 
support multiple migrate paths.

### Data structures

**Required**

All CW2-compliant contracts must store the following data:

* key: `contract_info`
* data: Json-serialized `ContractVersion`

```rust
pub struct ContractVersion {
    /// contract is a globally unique identifier for the contract.
    /// it should build off standard namespacing for whichever language it is in,
    /// and prefix it with the registry we use.
    /// For rust we prefix with `crates.io:`, to give us eg. `crates.io:cw20-base`
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
```

Thus, an serialized example may looks like:

```json
{
    "contract": "crates.io:cw20-base",
    "version": "v0.1.0",
    "supported_interface": ["crates.io:cw721","crates.io:cw2"]
}
```
