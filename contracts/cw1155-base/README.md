# CW1155 Basic

This is a basic implementation of a cw1155 contract.
It implements the [CW1155 spec](../../packages/cw1155/README.md) and manages multiple tokens
(fungible or non-fungible) under one contract.

## Instantiation

To create it, you must pass in a `minter` address.

```rust
#[cw_serde]
pub struct InstantiateMsg {
    /// The minter is the only one who can create new tokens.
    /// This is designed for a base token platform that is controlled by an external program or
    /// contract.
    pub minter: String,
}
```

## Messages

All other messages and queries are defined by the 
[CW1155 spec](../../packages/cw1155/README.md). Please refer to it for more info.