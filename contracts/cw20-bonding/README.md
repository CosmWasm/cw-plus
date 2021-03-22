# CW20 Bonding curve

This builds on the [Basic CW20 interface](../../packages/cw20/README.md)
as implemented in [`cw20-base`](../cw20-base/README.md).

This serves three purposes:

* A usable and extensible contract for arbitrary bonding curves
* A demonstration of how to extend `cw20-base` to add extra functionality
* A demonstration of the [Receiver interface]([Basic CW20 interface](../../packages/cw20/README.md#receiver))

## Design

There are two variants - accepting native tokens and accepting cw20 tokens
as the *reserve* token (this is the token that is input to the bonding curve).

Minting: When the input is sent to the contract (either via `ExecuteMsg::Buy{}`
with native tokens, or via `ExecuteMsg::Receive{}` with cw20 tokens),
those tokens remain on the contract and it issues it's own token to the
sender's account (known as *supply* token).

Burning: We override the burn function to not only burn the requested tokens,
but also release a proper number of the input tokens to the account that burnt
the custom token

Curves: `handle` specifies a bonding function, which is sent to parameterize
`handle_fn` (which does all the work). The curve is set when compiling
the contract. In fact many contracts can just wrap `cw20-bonding` and
specify the custom curve parameter.

Read more about [bonding curve math here](https://yos.io/2018/11/10/bonding-curves/)

Note: the first version only accepts native tokens as the 

### Math

Given a price curve `f(x)` = price of the `x`th token, we want to figure out
how to buy into and sell from the bonding curve. In fact we can look at
the total supply issued. let `F(x)` be the integral of `f(x)`. We have issued
`x` tokens for `F(x)` sent to the contract. Or, in reverse, if we send
`x` tokens to the contract, it will mint `F^-1(x)` tokens.

From this we can create some formulas. Assume we currently have issued `S`
tokens in exchange for `N = F(S)` input tokens. If someone sends us `x` tokens,
how much will we issue?

`F^-1(N+x) - F^-1(N)` = `F^-1(N+x) - S`

And if we sell `x` tokens, how much we will get out:

`F(S) - F(S-x)` = `N - F(S-x)`

Just one calculation each side. To be safe, make sure to round down and
always check against `F(S)` when using `F^-1(S)` to estimate how much
should be issued. This will also safely give us how many tokens to return.

There is built in support for safely [raising i128 to an integer power](https://doc.rust-lang.org/std/primitive.i128.html#method.checked_pow).
There is also a crate to [provide nth-root of for all integers](https://docs.rs/num-integer/0.1.43/num_integer/trait.Roots.html).
With these two, we can handle most math except for logs/exponents.

Compare this to [writing it all in solidity](https://github.com/OpenZeppelin/openzeppelin-contracts/blob/7b7ff729b82ea73ea168e495d9c94cb901ae95ce/contracts/math/Power.sol)

Examples:

Price Constant: `f(x) = k` and `F(x) = kx` and `F^-1(x) = x/k`

Price Linear: `f(x) = kx` and `F(x) = kx^2/2` and `F^-1(x) = (2x/k)^(0.5)`

Price Square Root: `f(x) = x^0.5` and `F(x) = x^1.5/1.5` and `F^-1(x) = (1.5*x)^(2/3)`

We will only implement these curves to start with, and leave it to others to import this with more complex curves,
such as logarithms.