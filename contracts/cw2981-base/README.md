# CW-2981 Token-level Royalties

An example of porting EIP-2981 to implement royalties at a token mint level.

All of the CW-721 logic and behaviour you would expect for an NFT is implemented as normal, but additionally at mint time, royalty information can be attached to a token.

Exposes two new query message types:

```rust
// Should be called on sale to see if royalties are owed
// by the marketplace selling the NFT.
// See https://eips.ethereum.org/EIPS/eip-2981
RoyaltyInfo {
    token_id: String,
    // the denom of this sale must also be the denom returned by RoyaltiesInfoResponse
    sale_price: u128,
},

// Called against the contract to signal that CW-2981 is implemented
CheckRoyalties {},
```
