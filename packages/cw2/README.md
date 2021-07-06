# CW2 Spec: Contract Info for Migration

Most of the CW* specs are focused on the *public interfaces*
of the contract. The APIs used for `ExecuteMsg` or `QueryMsg`.
However, when we wish to migrate from contract A to contract B,
contract B needs to be aware somehow of how the *state was encoded*.

Generally we use Singletons and Buckets to store the state, but
if I upgrade to a `cw20-with-bonding-curve` contract, it will only
work properly if I am migrating from a `cw20-base` contract. But how
can the new contract know what format the data was stored.

This is where CW2 comes in. It specifies on special Singleton to
be stored on disk by all contracts on `instantiate`. When the `migrate`
function is called, then the new contract can read that data and
see if this is an expected contract we can migrate from. And also
contain extra version information if we support multiple migrate
paths.

### Data structures

**Required**

All CW2-compliant contracts must store the following data:

* key: `\x00\x0dcontract_info` (length prefixed "contract_info" using Singleton pattern)
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
}
```

Thus, an serialized example may looks like:

```json
{
    "contract": "crates.io:cw20-base",
    "version": "v0.1.0"
}
```

### Queries

Since the state is well defined, we do not need to support any "smart queries".
We do provide a helper to construct a "raw query" to read the ContractInfo
of any CW2-compliant contract.
