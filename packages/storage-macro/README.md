# CW-Storage-Plus: Macro helper for storage package 

Procedural macros helper for interacting with cw-storage-plus and cosmwasm-storage.

## Current features

Auto generate IndexList impl for your indexes struct.

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct TestStruct {
    id: u64,
    id2: u32,
    addr: Addr,
}

#[index_list(TestStruct)] // <- Add this line right here,.
struct TestIndexes<'a> {
    id: MultiIndex<'a, u32, TestStruct, u64>,
    addr: UniqueIndex<'a, Addr, TestStruct>,
}
```
