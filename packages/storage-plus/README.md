# CW-Storage-Plus: Enhanced/experimental storage engines for CosmWasm

The ideas in here are based on the `cosmwasm-storage` crate. However,
after much usage, we decided a complete rewrite could allow us to add
more powerful and easy to use interfaces. Here are those interfaces.

**Status: experimental**

You currently should not be using this crate outside of the `cosmwasm-plus`
repo. This is a first draft of many types. We will update the status
after they have been used more heavily and the interfaces stabilized.

The ideas/desired functionality in here should be more or final, 
just the form to express them that is not final. As we add new functionality,
we will continue to refine the foundations, but maintain semver.

## Usage Overview

We introduce two main classes to provide a productive abstraction
on top of `cosmwasm_std::Storage`. They are `Item`, which is
a typed wrapper around one database key, providing some helper functions
for interacting with it without dealing with raw bytes. And `Map`,
which allows you to store multiple unique typed objects under a prefix,
indexed by a simple (`&[u8]`) or compound (eg. `(&[u8], &[u8])`) primary key.

These correspond to the concepts represented in `cosmwasm_storage` by
`Singleton` and `Bucket`, but with a re-designed API and implementation
to require less typing for developers and less gas usage in the contracts.

## Item

The usage of an [`Item`](./src/item.rs) is pretty straight-forward. 
You must simply provide the proper type, as well as a database key not 
used by any other item. Then it will provide you with a nice interface 
to interact with such data. 

If you are coming from using `Singleton`, the biggest change is that
we no longer store `Storage` inside, meaning we don't need read and write
variants of the object, just one type. Furthermore, we use `const fn` 
to create the `Item`, allowing it to be defined as a global compile-time
constant rather than a function that must be constructed each time,
which saves gas as well as typing.

Example Usage:

```rust
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Config {
    pub owner: String,
    pub max_tokens: i32,
}

// note const constructor rather than 2 functions with Singleton
const CONFIG: Item<Config> = Item::new(b"config");

fn demo() -> StdResult<()> {
    let mut store = MockStorage::new();

    // may_load returns Option<T>, so None if data is missing
    // load returns T and Err(StdError::NotFound{}) if data is missing
    let empty = CONFIG.may_load(&store)?;
    assert_eq!(None, empty);
    let cfg = Config {
        owner: "admin".to_string(),
        max_tokens: 1234,
    };
    CONFIG.save(&mut store, &cfg)?;
    let loaded = CONFIG.load(&store)?;
    assert_eq!(cfg, loaded);

    // update an item with a closure (includes read and write)
    // returns the newly saved value
    let output = CONFIG.update(&mut store, |mut c| -> StdResult<_> {
        c.max_tokens *= 2;
        Ok(c)
    })?;
    assert_eq!(2468, output.max_tokens);

    // you can error in an update and nothing is saved
    let failed = CONFIG.update(&mut store, |_| -> StdResult<_> {
        Err(StdError::generic_err("failure mode"))
    });
    assert!(failed.is_err());

    // loading data will show the first update was saved
    let loaded = CONFIG.load(&store)?;
    let expected = Config {
        owner: "admin".to_string(),
        max_tokens: 2468,
    };
    assert_eq!(expected, loaded);

    // we can remove data as well
    CONFIG.remove(&mut store);
    let empty = CONFIG.may_load(&store)?;
    assert_eq!(None, empty);

    Ok(())
}
```

## Map

The usage of an [`Map`](./src/item.rs) is a little more complex, but
is still pretty straight-forward. You can imagine it as a storage-backed
`BTreeMap`, allowing key-value lookups with typed values. In addition,
we support not only simple binary keys (`&[u8]`), but tuples, which are
combined. This allows us to store allowances as composite keys 
eg. `(owner, spender)` to look up the balance.

Beyond direct lookups, we have a super power not found in Ethereum -
iteration. That's right, you can list all items in a `Map`, or only
part of them. We can efficiently allow pagination over these items as
well, starting at the point the last query ended, with low gas costs.
This requires the `iterator` feature to be enabled in `cw-storage-plus`
(which automatically enables it in `cosmwasm-std` as well).

If you are coming from using `Bucket`, the biggest change is that
we no longer store `Storage` inside, meaning we don't need read and write
variants of the object, just one type. Furthermore, we use `const fn` 
to create the `Bucket`, allowing it to be defined as a global compile-time
constant rather than a function that must be constructed each time,
which saves gas as well as typing. In addition, the composite indexes
(tuples) is more ergonomic and expressive of intention, and the range
interface has been improved.

Here is an example with normal (simple) keys:

```rust
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct Data {
    pub name: String,
    pub age: i32,
}

const PEOPLE: Map<&[u8], Data> = Map::new(b"people");

fn demo() -> StdResult<()> {
    let mut store = MockStorage::new();
    let data = Data {
        name: "John".to_string(),
        age: 32,
    };

    // load and save with extra key argument
    let empty = PEOPLE.may_load(&store, b"john")?;
    assert_eq!(None, empty);
    PEOPLE.save(&mut store, b"john", &data)?;
    let loaded = PEOPLE.load(&store, b"john")?;
    assert_eq!(data, loaded);

    // nothing on another key
    let missing = PEOPLE.may_load(&store, b"jack")?;
    assert_eq!(None, missing);

    // update function for new or existing keys
    let birthday = |d: Option<Data>| -> StdResult<Data> {
        match d {
            Some(one) => Ok(Data {
                name: one.name,
                age: one.age + 1,
            }),
            None => Ok(Data {
                name: "Newborn".to_string(),
                age: 0,
            }),
        }
    };

    let old_john = PEOPLE.update(&mut store, b"john", birthday)?;
    assert_eq!(33, old_john.age);
    assert_eq!("John", old_john.name.as_str());

    let new_jack = PEOPLE.update(&mut store, b"jack", birthday)?;
    assert_eq!(0, new_jack.age);
    assert_eq!("Newborn", new_jack.name.as_str());

    // update also changes the store
    assert_eq!(old_john, PEOPLE.load(&store, b"john")?);
    assert_eq!(new_jack, PEOPLE.load(&store, b"jack")?);

    // removing leaves us empty
    PEOPLE.remove(&mut store, b"john");
    let empty = PEOPLE.may_load(&store, b"john")?;
    assert_eq!(None, empty);

    Ok(())
}
```

### Composite Keys

There are times when we want to use multiple items as a key, for example, when
storing allowances based on account owner and spender. We could try to manually
concatenate them before calling, but that can lead to overlap, and is a bit
low-level for us. Also, by explicitly separating the keys, we can easily provide
helpers to do range queries over a prefix, such as "show me all allowances for
one owner" (first part of the composite key). Just like you'd expect from your
favorite database.

Here how we use it with composite keys. Just define a tuple as a key and use that
everywhere you used a byte slice above.

```rust
// Note the tuple for primary key. We support one slice, or a 2 or 3-tuple
// adding longer tuples is quite easy but unlikely to be needed.
const ALLOWANCE: Map<(&[u8], &[u8]), u64> = Map::new(b"allow");

fn demo() -> StdResult<()> {
    let mut store = MockStorage::new();

    // save and load on a composite key
    let empty = ALLOWANCE.may_load(&store, (b"owner", b"spender"))?;
    assert_eq!(None, empty);
    ALLOWANCE.save(&mut store, (b"owner", b"spender"), &777)?;
    let loaded = ALLOWANCE.load(&store, (b"owner", b"spender"))?;
    assert_eq!(777, loaded);

    // doesn't appear under other key (even if a concat would be the same)
    let different = ALLOWANCE.may_load(&store, (b"owners", b"pender")).unwrap();
    assert_eq!(None, different);

    // simple update
    ALLOWANCE.update(&mut store, (b"owner", b"spender"), |v| {
        Ok(v.unwrap_or_default() + 222)
    })?;
    let loaded = ALLOWANCE.load(&store, (b"owner", b"spender"))?;
    assert_eq!(999, loaded);

    Ok(())
}
```

### Path

Under the scenes, we create a `Path` from the `Map` when accessing a key.
`PEOPLE.load(&store, b"jack") == PEOPLE.key(b"jack").load()`.
`Map.key()` returns a `Path`, which has the same interface as `Item`,
reusing the calculated path to this key.

For simple keys, this is just a bit less typing and a bit less gas if you 
use the same key for many calls. However, for composite keys, like 
`(b"owner", b"spender")` it is **much** less typing. And highly recommended anywhere 
you will use the a composite key even twice:

```rust
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct Data {
    pub name: String,
    pub age: i32,
}

const PEOPLE: Map<&[u8], Data> = Map::new(b"people");
const ALLOWANCE: Map<(&[u8], &[u8]), u64> = Map::new(b"allow");

fn demo() -> StdResult<()> {
    let mut store = MockStorage::new();
    let data = Data {
        name: "John".to_string(),
        age: 32,
    };

    // create a Path one time to use below
    let john = PEOPLE.key(b"john");

    // Use this just like an Item above
    let empty = john.may_load(&store)?;
    assert_eq!(None, empty);
    john.save(&mut store, &data)?;
    let loaded = john.load(&store)?;
    assert_eq!(data, loaded);
    john.remove(&mut store);
    let empty = john.may_load(&store)?;
    assert_eq!(None, empty);

    // Same for composite keys, just use both parts in key().
    // Notice how much less verbose than the above example.
    let allow = ALLOWANCE.key((b"owner", b"spender"));
    allow.save(&mut store, &1234)?;
    let loaded = allow.load(&store)?;
    assert_eq!(1234, loaded);
    allow.update(&mut store, |x| Ok(x.unwrap_or_default() * 2))?;
    let loaded = allow.load(&store)?;
    assert_eq!(2468, loaded);

    Ok(())
}
```

### Prefix 

In addition to getting one particular item out of a map, we can iterate over the map
(or a subset of the map). This let's us answer questions like "show me all tokens",
and we provide some nice `Bound`s helpers to easily allow pagination or custom ranges.

The general format is to get a `Prefix` by calling `map.prefix(k)`, where `k` is exactly
one less item than the normal key (If `map.key()` took `(&[u8], &[u8])`, then `map.prefix()` takes `&[u8]`.
If `map.key()` took `&[u8]`, `map.prefix()` takes `()`). Once we have a prefix space, we can iterate
over all items with `range(store, min, max, order)`. It supports `Order::Ascending` or `Order::Descending`.
`min` is the lower bound and `max` is the higher bound.

```rust
#[derive(Copy, Clone, Debug)]
pub enum Bound<'a> {
    Inclusive(&'a [u8]),
    Exclusive(&'a [u8]),
    None,
}
```

If the `min` and `max` bounds, it will return all items under this prefix. You can use `.take(n)` to
limit the results to `n` items and start doing pagination. You can also set the `min` bound to
eg. `Bound::Exclusive(last_value)` to start iterating over all items *after* the last value. Combined with
`take`, we easily have pagination support. You can also use `Bound::Inclusive(x)` when you want to include any
perfect matches. To better understand the API, please read the following example:

```rust
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
struct Data {
    pub name: String,
    pub age: i32,
}

const PEOPLE: Map<&[u8], Data> = Map::new(b"people");
const ALLOWANCE: Map<(&[u8], &[u8]), u64> = Map::new(b"allow");

fn demo() -> StdResult<()> {
    let mut store = MockStorage::new();

    // save and load on two keys
    let data = Data { name: "John".to_string(), age: 32 };
    PEOPLE.save(&mut store, b"john", &data)?;
    let data2 = Data { name: "Jim".to_string(), age: 44 };
    PEOPLE.save(&mut store, b"jim", &data2)?;

    // iterate over them all
    let all: StdResult<Vec<_>> = PEOPLE
        .range(&store, Bound::None, Bound::None, Order::Ascending)
        .collect();
    assert_eq!(
        all?,
        vec![(b"jim".to_vec(), data2), (b"john".to_vec(), data.clone())]
    );

    // or just show what is after jim
    let all: StdResult<Vec<_>> = PEOPLE
        .range(
            &store,
            Bound::Exclusive(b"jim"),
            Bound::None,
            Order::Ascending,
        )
        .collect();
    assert_eq!(all?, vec![(b"john".to_vec(), data)]);

    // save and load on three keys, one under different owner
    ALLOWANCE.save(&mut store, (b"owner", b"spender"), &1000)?;
    ALLOWANCE.save(&mut store, (b"owner", b"spender2"), &3000)?;
    ALLOWANCE.save(&mut store, (b"owner2", b"spender"), &5000)?;

    // get all under one key
    let all: StdResult<Vec<_>> = ALLOWANCE
        .prefix(b"owner")
        .range(&store, Bound::None, Bound::None, Order::Ascending)
        .collect();
    assert_eq!(
        all?,
        vec![(b"spender".to_vec(), 1000), (b"spender2".to_vec(), 3000)]
    );

    // Or ranges between two items (even reverse)
    let all: StdResult<Vec<_>> = ALLOWANCE
        .prefix(b"owner")
        .range(
            &store,
            Bound::Exclusive(b"spender1"),
            Bound::Inclusive(b"spender2"),
            Order::Descending,
        )
        .collect();
    assert_eq!(all?, vec![(b"spender2".to_vec(), 3000)]);

    Ok(())
}
```

## Indexed Map

TODO: we are working on a version of a map that manages multiple
secondary indexed transparently. That work is coming soon.
