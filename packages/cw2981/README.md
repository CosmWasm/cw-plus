# CW-2981 Spec: NFT Royalties

CW-2981 is an un-opinionated minimal way of implementing royalties for NFTs, based on the [EIP-2981 NFT Royalty Standard](https://eips.ethereum.org/EIPS/eip-2981).

Note that this package seeks to show the base implementation. As it is primarily a query interface, with the implementation left up to the contract author, more complicated patterns, like a time-decay (presumably tied to block height) on royalties, could be implemented.

In order to achieve this, you would simply use the Queries defined here, implement custom Messages to store the required state, and custom query handlers to return the correct royalty to clients and marketplaces.

### Messages

There are two patterns for implementation provided here:

1. Token-level royalties.
If you want to implement token-level royalties, use a normal CW-721 instantiation message, and simply return `true` from a query handler matched to `CheckRoyalties` to signal that the contract implements CW-2981.
2. Contract-level royalties.
If you want all tokens minted by the contract to have the same royalty info, use `ContractRoyaltiesInstantiateMsg` to provide these parameters. You can set `royalty_payments` to `false` and provide `None` for the other parameters, and the contract will function as a normal CW-721.

As alluded to above, you could also extend these examples with additional fields to provide custom math for the royalty percentage. The key is that the interface exposed by the queries (below) must be the same.

### Queries

Whether or not you are implementing royalties at contract-level or token level, two queries are defined:

1. `RoyaltyInfo` this takes a `token_id` and `sale_price`, and should return `RoyaltiesInfoResponse`. It is denomination-agnostic, the assumption being that the same denomination that is passed in should be returned. This also means it supports e.g. the NFT being sold for a price in CW20.
2. `CheckRoyalties` signals that this contract implements CW-2981 and thus marketplaces should check tokens on sale.


