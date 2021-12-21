# Migrating

This guide lists API changes between *cw-plus* major releases.

## v0.10.3 -> v0.11.0

### Breaking Issues / PRs

- Rename cw0 to utils [\#471](https://github.com/CosmWasm/cw-plus/issues/471) / Cw0 rename [\#508](https://github.com/CosmWasm/cw-plus/pull/508)

The `cw0` package was renamed to `utils`. The required changes are straightforward:

```diff
diff --git a/contracts/cw1-subkeys/Cargo.toml b/contracts/cw1-subkeys/Cargo.toml
index 1924b655..37af477d 100644
--- a/contracts/cw1-subkeys/Cargo.toml
+++ b/contracts/cw1-subkeys/Cargo.toml
@@ -19,7 +19,7 @@ library = []
 test-utils = []

 [dependencies]
-cw0 = { path = "../../packages/cw0", version = "0.10.3" }
+utils = { path = "../../packages/utils", version = "0.10.3" }
 cw1 = { path = "../../packages/cw1", version = "0.10.3" }
 cw2 = { path = "../../packages/cw2", version = "0.10.3" }
 cw1-whitelist = { path = "../cw1-whitelist", version = "0.10.3", features = ["library"] }
```

```diff
diff --git a/contracts/cw1-subkeys/src/contract.rs b/contracts/cw1-subkeys/src/contract.rs
index b4852225..f20a65ec 100644
--- a/contracts/cw1-subkeys/src/contract.rs
+++ b/contracts/cw1-subkeys/src/contract.rs
@@ -8,7 +8,6 @@ use cosmwasm_std::{
     ensure, ensure_ne, to_binary, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg,
     Empty, Env, MessageInfo, Order, Response, StakingMsg, StdResult,
 };
-use cw0::Expiration;
 use cw1::CanExecuteResponse;
 use cw1_whitelist::{
     contract::{
@@ -21,6 +20,7 @@ use cw1_whitelist::{
 use cw2::{get_contract_version, set_contract_version};
 use cw_storage_plus::Bound;
 use semver::Version;
+use utils::Expiration;

 use crate::error::ContractError;
 use crate::msg::{
```

- Deprecate `range` to `range_raw` [\#460](https://github.com/CosmWasm/cw-plus/issues/460) /
`range` to `range raw` [\#576](https://github.com/CosmWasm/cw-plus/pull/576).

`range` was renamed to `range_raw` (no key deserialization), and `range_de` to `range`. This means you can now count
on the key to be deserialized for you, which results in cleaner / simpler code.

There are many examples in the contracts code base, by example:
```diff

```

If / when you don't need key deserialization, just can just rename `range` to `range_raw` and you are good to go.

If you need / want key deserialization for **indexes**, you need to specify the primary key type as a last (optional)
argument in the index key specification. If not specified, it defaults to `()`, which means that primary keys will
not only not be deserialized, but also not provided. This is for backwards-compatibility with current indexes
specifications, and may change in the future once these features are stabilized.

Also, as part of this issue, `keys` was renamed to `keys_raw`, and `keys_de` to `keys`. `prefix_range` was also renamed,
to `prefix_range_raw`, and its key deserialization counterpart (`prefix_range_de`) to `prefix_range` in turn.

Finally, this applies to all the `Map`-like types, including indexed maps.

- `UniqueIndex` / `MultiIndex` key consistency [\#532](https://github.com/CosmWasm/cw-plus/issues/532) /
Index keys consistency [\#568](https://github.com/CosmWasm/cw-plus/pull/568)

For both `UniqueIndex` and `MultiIndex`, returned keys (deserialized or not) are now the associated *primary keys* of
the data. This is a breaking change for `MultiIndex`, as the previous version returned a composite of the index key and
the primary key.

Required changes are:
```diff
```

- Remove the primary key from the `MultiIndex` key specification [\#533](https://github.com/CosmWasm/cw-plus/issues/533) /
`MultiIndex` primary key spec removal [\#569](https://github.com/CosmWasm/cw-plus/pull/569)

The primary key is no longer needed when specifying `MultiIndex` keys. This simplifies both `MultiIndex` definitions
and usage.

Required changes are along the lines of:
```diff
```

- Incorrect I32Key Index Ordering [\#489](https://github.com/CosmWasm/cw-plus/issues/489) /
Signed int keys order [\#582](https://github.com/CosmWasm/cw-plus/pull/582)

As part of range iterators revamping, we fixed the order of signed integer keys. You shouldn't change anything in your
code base for this, but if you were using signed keys and relying on their ordering, that has now changed for the better.
Take into account also that **the internal representation of signed integer keys has changed**. So, if you
have data stored under signed integer keys you would need to **migrate it**, or recreate it under the new representation.

As part of this, a couple helpers for handling int keys serialization and deserialization were introduced:
 - `from_cw_bytes` Integer (signed and unsigned) values deserialization.
 - `to_cw_bytes` - Integer (signed and unsigned) values serialization.

You shouldn't need these, except when manually handling raw integer keys serialization / deserialization.

### Non-breaking Issues / PRs

- Deprecate `IntKey` [\#472](https://github.com/CosmWasm/cw-plus/issues/472) /
Deprecate IntKey [\#547](https://github.com/CosmWasm/cw-plus/pull/547)

See [CHANGELOG.md](CHANGELOG.md) for the full list.
