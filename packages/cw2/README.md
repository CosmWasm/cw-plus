# CW2 Spec: Contract Info for Migration

Most of the CW* specs are focused on the *public interfaces*
of the contract. The APIs used for `HandleMsg` or `QueryMsg`.
However, when we wish to migrate from contract A to contract B,
contract B needs to be aware somehow of how the *state was encoded*.

Generally we use Singletons and Buckets to store the state, but
if I upgrade to a `cw20-with-bonding-curve` contract, it will only
work properly if I am migrating from a `cw20-base` contract. But how
can the new contract know what format the data was stored.

This is where CW2 comes in. It specifies on special Singleton to
be stored on disk by all contracts on `init`. When the `migrate`
function is called, then the new contract can read that data and
see if this is an expected contract we can migrate from. And also
contain extra version information if we support multiple migrate
paths.

### Data structures

**Required**

All CW2-compliant contracts must store the following data:

* key: `\x00\x0dcontract_info` (length prefixed "contract_info" using Singleton pattern)
* data: Json-serialized `ContractInfo`

```rust
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

pub struct ContractInfo {
    versions: Vec<ContractVersion>,
}
```

Thus, an serialized example may looks like:

```json
{
  "versions": [{
    "contract": "crate:cw20-base",
    "version": "v0.1.0"
  }]
}
```

Note we explicitly allow multiple version for mix-ins. For example,
`cw1-subkeys` imports the config/admin list bucket from `cw1-whitelist`
as well as adding some more info to it. We may want to explicitly mention
the source of the state to be migrated:

```json
{
  "versions": [{
    "contract": "crate:cw1-whitelist",
    "version": "v0.1.0"
  }, {
    "contract": "crate:cw1-subkeys",
    "version": "v0.1.0"
  }]
}
```

### Queries

Since the state is well defined, we do not need to support any "smart queries".
We do provide a helper to construct a "raw query" to read the ContractInfo
of any CW2-compliant contract.
