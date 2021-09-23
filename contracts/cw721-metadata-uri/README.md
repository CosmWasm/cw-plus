# CW721 Metadata URI

In Ethereum, the ERC721 standard includes a metadata_uri field to store all metadata offchain.
With CW721-Base in CosmWasm, we allow you to store any data on chain you wish, using a generic `extension: T`.

In order to provide better "out of the box" compatibility for people migrating from the Ethereum ecosystem,
and to demonstrate how to use the extension ability, we have created this simple contract. There is no business
logic here, but looking at `lib.rs` will show you how do define custom data that is included when minting and
available in all queries.

In particular, here we define:

```rust
pub struct Extension {
    pub metadata_uri: String,
}
```

This means when you query `NftInfo{name: "Enterprise"}`, you will get something like:

```json
{
  "name": "Enterprise",
  "description": "USS Starship Enterprise",
  "image": null,
  "extension": {
    "metadata_uri": "http://starships.example.com/Starships/Enterprise.json"
  }
}
```

Please look at the test code for an example usage in Rust.

## Notice

Feel free to use this contract out of the box, or as inspiration for further customization of cw721-base.
We will not be adding new features or business logic here.
