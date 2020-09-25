# Contracts

Here are a number of useful contracts that either implement or consume
the interfaces defined in `packages/cw*`.

## Creating a new contract

Use [`cosmwasm-template`](https://github.com/CosmWasm/cosmwasm-template) as a
basis, in particular the `cosmwasm-plus` branch.

```bash
cd contracts
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --branch cosmwasm-plus --name PROJECT_NAME
cd PROJECT_NAME
rm -rf .git
```