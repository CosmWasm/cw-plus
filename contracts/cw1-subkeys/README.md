# CW1 Subkeys

This builds on `cw1-whitelist` to provide the first non-trivial solution.
It still works like `cw1-whitelist` with a set of admins (typically 1)
which have full control of the account. However, you can then grant
a number of accounts allowances to send native tokens from this account.

This was proposed in Summer 2019 for the Cosmos Hub and resembles the
functionality of ERC20 (allowances and transfer from).

## Details

Basically, any admin can add an allowance for a `(spender, denom)` pair
(similar to cw20 `IncreaseAllowance` / `DecreaseAllowance`). Any non-admin
account can try to execute a `CosmosMsg::Bank(BankMsg::Send{})` from this
contract and if they have the required allowances, their allowance will be
reduced and the send message relayed. If they don't have sufficient authorization,
or if they try to proxy any other message type, then the attempt will be rejected.

### Messages

This adds 2 messages beyond the `cw1` spec:

```rust
enum HandleMsg {
    IncreaseAllowance {
        spender: HumanAddr,
        denom: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    DecreaseAllowance {
        spender: HumanAddr,
        denom: String,
        amount: Uint128,
        expires: Option<Expiration>,
    }
}
```

### Queries

It also adds one more query type:

```rust
enum QueryMsg {
    Allowance {
        spender: HumanAddr,
    }
}

pub struct AllowanceResponse {
    pub allowance: Vec<Coin>,
    pub expires: Expiration,
}
```

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via:

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw1_subkeys.wasm .
ls -l cw1_subkeys.wasm
sha256sum cw1_subkeys.wasm
```

Or for a production-ready (compressed) build, run the following from the
repository root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="cosmwasm_plus_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.10.2
```

The optimized contracts are generated in the `artifacts/` directory.
