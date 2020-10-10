# CW-Storage-Plus: Enhanced/experimental storage engines for CosmWasm

The ideas in here are based on the `cosmwasm-storage` crate. However,
after much usage, we decided a complete rewrite could allow us to add
more powerful and easy touse interfaces. Here are those interfaces.

**Status: experimental**

You currently should not be using this crate outside of the `cosmwasm-plus`
repo. This is a first draft of many types. We will update the status
after they have been used more heavily and the interfaces stabilized.

The ideas/desired functionality in here should be more or final, 
just the form to express them which will keep changing.

## Usage Overview

We introduce two main classes to provide a productive abstraction
on top of `cosmwasm_std::Storage`. They are `Item`, which is
a typed wrapper around one database key, providing some helper functions
for interacting with it without dealing with raw bytes. And `Map`,
which allows you to store multiple typed objects under a prefix,
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
    CONFIG.save(&mut store, &cfg);
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
    let failed = CONFIG.update(&mut store, |mut c| -> StdResult<_> {
        Err(StdError::generic_err("failure mode"))
    });
    assert!(failed.is_err());

    // loading data will show the first update was saved
    let loaded = CONFIG.load(&store)?;
    let expected = Config {
        owner: "admin".to_string,
        max_tokens: 2468,
    };
    assert_eq!(expected, loaded);
    
    // we can remove data as well
    CONFIG.remove(&mut store);
    let empty = CONFIG.may_load(&store)?;
    assert_eq!(None, empty);
}
```

## Map

**TODO**

### Path

**TODO**

### Prefix 

**TODO**

## Indexed Map

TODO: we are working on a version of a map that manages multiple
secondary indexed transparently. That work is coming soon.