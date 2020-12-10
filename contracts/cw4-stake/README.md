# CW4 Stake

This is a second implementation of the [cw4 spec](../../packages/cw4/README.md).
It fufills all elements of the spec, including the raw query lookups,
and it designed to be used as a backing storage for 
[cw3 compliant contracts](../../packages/cw3/README.md).

It provides a similar API to [`cw4-group`] (which handles elected membership),
but rather than appointing members (by admin or multisig), their
membership and weight is based on the number of tokens they have staked.
This is similar to many DAOs.

Only one denom can be bonded with both `min_bond` as the minimum amount
that must be sent by one address to enter, as well as `tokens_per_weight`,
which can be used to normalize the weight (eg. if the token is uatom
and you want 1 weight per ATOM, you can set `tokens_per_weight = 1_000_000`).

There is also an unbonding period (`Duration`) which sets how long the
tokens are frozen before being released. These frozen tokens can neither
be used for voting, nor claimed by the original owner. Only after the period
can you get your tokens back. This liquidity loss is the "skin in the game"
provided by staking to this contract.

## Init

**TODO**

To create it, you must pass in a list of members, as well as an optional
`admin`, if you wish it to be mutable.

```rust
pub struct InitMsg {
    pub admin: Option<HumanAddr>,
    pub members: Vec<Member>,
}

pub struct Member {
    pub addr: HumanAddr,
    pub weight: u64,
}
```

Members are defined by an address and a weight. This is transformed
and stored under their `CanonicalAddr`, in a format defined in
[cw4 raw queries](../../packages/cw4/README.md#raw).

Note that 0 *is an allowed weight*. This doesn't give any voting rights, but
it does define this address is part of the group. This could be used in
eg. a KYC whitelist to say they are allowed, but cannot participate in
decision-making.

## Messages

Update messages and queries are defined by the 
[cw4 spec](../../packages/cw4/README.md). Please refer to it for more info.