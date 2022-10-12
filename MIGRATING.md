# Migrating

This guide lists API changes between *cw-plus* major releases.

## v0.13.x -> v0.14.0

### Breaking Issues / PRs

- `MultiIndex` index fn params now include the pk [\#670](https://github.com/CosmWasm/cw-plus/issues/670)

The `idx_fn` param of the `MultiIndex` constructor now has signature `fn(&[u8], &T) -> IK`, where the first param is the
primary key of the index (in raw format), and the second param is the associated value.
That allows us to use the pk or parts of it for building the index key.

Migration of existing code is straight-forward. Just add an (unused) `_pk` param to the index function definition:

```diff
fn build_map<'a>() -> IndexedMap<'a, &'a str, Data, DataIndexes<'a>> {
   let indexes = DataIndexes {
-     name: MultiIndex::new(|d| d.name.clone(), "data", "data__name"),
+     name: MultiIndex::new(|_pk, d| d.name.clone(), "data", "data__name"),
      age: UniqueIndex::new(|d| d.age, "data__age"),
      name_lastname: UniqueIndex::new(
```

If you want to leverage this new functionality, take a look at the `pk_based_index()` test / example
in `src/indexed_map.rs`.

## v0.11.0 -> v0.12.0

### Breaking Issues / PRs

- Type safe `Bound`s [\#462](https://github.com/CosmWasm/cw-plus/issues/462)

Bounds are now type-safe. That means the bound type must match the key / sub-key you are ranging over.

Migration code example:

```diff
fn list_allowed(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListAllowedResponse> {
     let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
-    let start = match start_after {
-        Some(x) => Some(Bound::exclusive(deps.api.addr_validate(&x)?.into_string())),
-        None => None,
-    };
+    let addr = maybe_addr(deps.api, start_after)?;
+    let start = addr.as_ref().map(Bound::exclusive);

     let allow = ALLOW_LIST
         .range(deps.storage, start, None, Order::Ascending)
```
Here the `ALLOW_LIST` key is of type `&Addr`. That's why we use `as_ref()` before the `map()` that builds the bound.
Notice also that this "forces" you to use `addr_validate()`, in order to build a bound over the proper type.

You can still use untyped bounds, with the `ExclusiveRaw` and `InclusiveRaw` enum types.
Migration code example, in case you want to keep your raw bounds:

```diff
pub fn query_all_allowances(
    deps: Deps,
    env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
     let limit = calc_limit(limit);
     // we use raw addresses here....
-    let start = start_after.map(Bound::exclusive);
+    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into()));

     let allowances = ALLOWANCES
         .range(deps.storage, start, None, Order::Ascending)
```
Notice that here we build a bound for an address, and using a raw bound allows us to skip address validation / build up.

See storage-plus [README.md](./packages/storage-plus/README.md#Bound) for more information on `Bound`.

## v0.10.3 -> v0.11.0

### Breaking Issues / PRs

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

Migration code example:
```rust
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: Empty) -> Result<Response, ContractError> {
  let version: Version = CONTRACT_VERSION.parse()?;
  let storage_version: Version = get_contract_version(deps.storage)?.version.parse()?;

  if storage_version < version {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Do the migration
    // Original map
    let signed_int_map: Map<IntKeyOld<i8>, String> = Map::new("signed_int_map");

    // New map (using a different namespace for safety. It could be the same with enough care)
    let signed_int_map_new: Map<i8, String> = Map::new("signed_int_map-v2");

    // Obtain all current keys (this will need to be paginated if there are many entries,
    // i.e. i32 or i64 instead of i8).
    // This may be gas intensive
    let current = signed_int_map
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

    // Store length for quality control (adjust if paginated)
    let current_count = current.len();

    // Remove the old map keys
    for (k, _) in current.iter() {
      signed_int_map.remove(deps.storage, (*k).into());
    }

    // Save in new format
    for (k, v) in current.into_iter() {
      signed_int_map_new.save(deps.storage, k, &v)?;
    }

    // Confirm old map is empty
    if signed_int_map
            .keys_raw(deps.storage, None, None, Order::Ascending)
            .next()
            .is_some()
    {
      return Err(StdError::generic_err("Original still not empty!").into());
    }

    // Obtain new keys, and confirm their amount.
    // May be gas intensive.
    let new_count = signed_int_map_new
            .keys_raw(deps.storage, None, None, Order::Ascending)
            .count();

    if current_count != new_count {
      return Err(StdError::generic_err(format!(
        "Current ({}) and new ({}) counts differ!",
        current_count, new_count
      ))
              .into());
    }
  }

  Ok(Response::new())
}
```

---

- Rename cw0 to utils [\#471](https://github.com/CosmWasm/cw-plus/issues/471) / Cw0 rename [\#508](https://github.com/CosmWasm/cw-plus/pull/508)

The `cw0` package was renamed to `cw-utils`. The required changes are straightforward:

```diff
diff --git a/contracts/cw1-subkeys/Cargo.toml b/contracts/cw1-subkeys/Cargo.toml
index 1924b655..37af477d 100644
--- a/contracts/cw1-subkeys/Cargo.toml
+++ b/contracts/cw1-subkeys/Cargo.toml
-cw0 = { path = "../../packages/cw0", version = "0.10.3" }
+cw-utils = { path = "../../packages/utils", version = "0.10.3" }
```

```diff
diff --git a/contracts/cw1-subkeys/src/contract.rs b/contracts/cw1-subkeys/src/contract.rs
index b4852225..f20a65ec 100644
--- a/contracts/cw1-subkeys/src/contract.rs
+++ b/contracts/cw1-subkeys/src/contract.rs
-use cw0::Expiration;
+use cw_utils::Expiration;
```

---

- Deprecate `range` to `range_raw` [\#460](https://github.com/CosmWasm/cw-plus/issues/460) /
`range` to `range raw` [\#576](https://github.com/CosmWasm/cw-plus/pull/576).

`range` was renamed to `range_raw` (no key deserialization), and `range_de` to `range`. This means you can now count
on the key to be deserialized for you, which results in cleaner / simpler code.

There are some examples in the contracts code base, by example:
```diff
diff --git a/contracts/cw3-fixed-multisig/src/contract.rs b/contracts/cw3-fixed-multisig/src/contract.rs
index 48a60083..2f2ef70d 100644
--- a/contracts/cw3-fixed-multisig/src/contract.rs
+++ b/contracts/cw3-fixed-multisig/src/contract.rs
@@ -385,21 +384,20 @@ fn list_votes(
         .range(deps.storage, start, None, Order::Ascending)
         .take(limit)
         .map(|item| {
-            let (key, ballot) = item?;
-            Ok(VoteInfo {
-                voter: String::from_utf8(key)?,
+            item.map(|(addr, ballot)| VoteInfo {
+                voter: addr.into(),
                 vote: ballot.vote,
                 weight: ballot.weight,
             })
```

If you don't need key deserialization, just can just rename `range` to `range_raw` and you are good to go.

If you want key deserialization for **indexes**, you need to specify the primary key type as a last (optional)
argument in the index key specification. If not specified, it defaults to `()`, which means that primary keys will
not only not be deserialized, but also not provided. This is for backwards-compatibility with current indexes
specifications, and may change in the future once these features are stabilized.
See `packages/storage-plus/src/indexed_map.rs` tests for reference.

---

Renamed methods:
- `range` -> `range_raw`
- `keys` -> `keys_raw`
- `prefix_range` -> `prefix_range_raw`
- `range_de` -> `range`
- `keys_de` -> `keys`
- `prefix_range_de` -> `prefix_range`

Finally, this applies to all the `Map`-like types, including indexed maps.

---

- `UniqueIndex` / `MultiIndex` key consistency [\#532](https://github.com/CosmWasm/cw-plus/issues/532) /
Index keys consistency [\#568](https://github.com/CosmWasm/cw-plus/pull/568)

For both `UniqueIndex` and `MultiIndex`, returned keys (deserialized or not) are now the associated *primary keys* of
the data. This is a breaking change for `MultiIndex`, as the previous version returned a composite of the index key and
the primary key.

Examples of required changes are:
```diff
diff --git a/packages/storage-plus/src/indexed_map.rs b/packages/storage-plus/src/indexed_map.rs
index 9f7178af..d11d501e 100644
--- a/packages/storage-plus/src/indexed_map.rs
+++ b/packages/storage-plus/src/indexed_map.rs
@@ -722,7 +722,7 @@ mod test {
             last_name: "".to_string(),
             age: 42,
         };
-        let pk1: &[u8] = b"5627";
+        let pk1: &str = "5627";
         map.save(&mut store, pk1, &data1).unwrap();

         let data2 = Data {
@@ -730,7 +730,7 @@ mod test {
             last_name: "Perez".to_string(),
             age: 13,
         };
-        let pk2: &[u8] = b"5628";
+        let pk2: &str = "5628";
         map.save(&mut store, pk2, &data2).unwrap();

         let data3 = Data {
@@ -738,7 +738,7 @@ mod test {
             last_name: "Young".to_string(),
             age: 24,
         };
-        let pk3: &[u8] = b"5629";
+        let pk3: &str = "5629";
         map.save(&mut store, pk3, &data3).unwrap();

         let data4 = Data {
@@ -746,7 +746,7 @@ mod test {
             last_name: "Bemberg".to_string(),
             age: 43,
         };
-        let pk4: &[u8] = b"5630";
+        let pk4: &str = "5630";
         map.save(&mut store, pk4, &data4).unwrap();

         let marias: Vec<_> = map
@@ -760,8 +760,8 @@ mod test {
         assert_eq!(2, count);

         // Remaining part (age) of the index keys, plus pks (bytes) (sorted by age descending)
-        assert_eq!((42, pk1.to_vec()), marias[0].0);
-        assert_eq!((24, pk3.to_vec()), marias[1].0);
+        assert_eq!(pk1, marias[0].0);
+        assert_eq!(pk3, marias[1].0);

         // Data
         assert_eq!(data1, marias[0].1);
```

---

- Remove the primary key from the `MultiIndex` key specification [\#533](https://github.com/CosmWasm/cw-plus/issues/533) /
`MultiIndex` primary key spec removal [\#569](https://github.com/CosmWasm/cw-plus/pull/569)

The primary key is no longer needed when specifying `MultiIndex` keys. This simplifies both `MultiIndex` definitions
and usage.

Required changes are along the lines of:
```diff
diff --git a/packages/storage-plus/src/indexed_map.rs b/packages/storage-plus/src/indexed_map.rs
index 022a4504..c7a3bb9d 100644
--- a/packages/storage-plus/src/indexed_map.rs
+++ b/packages/storage-plus/src/indexed_map.rs
@@ -295,8 +295,8 @@ mod test {
     }

     struct DataIndexes<'a> {
-        // Second arg is for storing pk
-        pub name: MultiIndex<'a, (String, String), Data, String>,
+        // Last type parameters are for signaling pk deserialization
+        pub name: MultiIndex<'a, String, Data, String>,
         pub age: UniqueIndex<'a, u32, Data, String>,
         pub name_lastname: UniqueIndex<'a, (Vec<u8>, Vec<u8>), Data, String>,
     }
@@ -326,11 +326,7 @@ mod test {
     // Can we make it easier to define this? (less wordy generic)
     fn build_map<'a>() -> IndexedMap<'a, &'a str, Data, DataIndexes<'a>> {
         let indexes = DataIndexes {
-            name: MultiIndex::new(
-                |d, k| (d.name.clone(), unsafe { String::from_utf8_unchecked(k) }),
-                "data",
-                "data__name",
-            ),
+            name: MultiIndex::new(|d| d.name.clone(), "data", "data__name"),
             age: UniqueIndex::new(|d| d.age, "data__age"),
             name_lastname: UniqueIndex::new(
                 |d| index_string_tuple(&d.name, &d.last_name),
@@ -469,8 +462,8 @@ mod test {
         // index_key() over MultiIndex works (empty pk)
         // In a MultiIndex, an index key is composed by the index and the primary key.
         // Primary key may be empty (so that to iterate over all elements that match just the index)
-        let key = ("Maria".to_string(), "".to_string());
-        // Use the index_key() helper to build the (raw) index key
+        let key = "Maria".to_string();
+        // Use the index_key() helper to build the (raw) index key with an empty pk
         let key = map.idx.name.index_key(key);
         // Iterate using a bound over the raw key
         let count = map
```

---

- `cw3-fixed-multisig` requires threshold during instantiation instead of `required_weight` parameter

`Threshold` type was moved to `packages/utils` along with surrounding implementations like `ThresholdResponse` etc.

```diff
use cw_utils::Threshold;

pub struct InstantiateMsg {
    pub voters: Vec<Voter>,
-   pub required_weight: u64,
+   pub threshold: Threshold,
    pub max_voting_period: Duration,
}
```

### Non-breaking Issues / PRs

- Deprecate `IntKey` [\#472](https://github.com/CosmWasm/cw-plus/issues/472) /
Deprecate IntKey [\#547](https://github.com/CosmWasm/cw-plus/pull/547)

The `IntKey` wrapper and its type aliases are marked as deprecated, and will be removed in the next major version.
The migration is straightforward, just remove the wrappers:
```diff
diff --git a/contracts/cw20-merkle-airdrop/src/contract.rs b/contracts/cw20-merkle-airdrop/src/contract.rs
index e8d5ea57..bdf555e8 100644
--- a/contracts/cw20-merkle-airdrop/src/contract.rs
+++ b/contracts/cw20-merkle-airdrop/src/contract.rs
@@ -6,7 +6,6 @@ use cosmwasm_std::{
 };
 use cw2::{get_contract_version, set_contract_version};
 use cw20::Cw20ExecuteMsg;
-use cw_storage_plus::U8Key;
 use sha2::Digest;
 use std::convert::TryInto;

@@ -113,7 +112,7 @@ pub fn execute_register_merkle_root(

     let stage = LATEST_STAGE.update(deps.storage, |stage| -> StdResult<_> { Ok(stage + 1) })?;

-    MERKLE_ROOT.save(deps.storage, U8Key::from(stage), &merkle_root)?;
+    MERKLE_ROOT.save(deps.storage, stage, &merkle_root)?;
     LATEST_STAGE.save(deps.storage, &stage)?;

     Ok(Response::new().add_attributes(vec![
```

- See [CHANGELOG.md](CHANGELOG.md) for the full list of non-breaking changes.
