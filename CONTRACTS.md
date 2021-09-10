# Contracts

Here are a number of useful contracts that either implement or consume
the interfaces defined in `packages/cw*`.

## Creating a new contract

Use [`cosmwasm-template`](https://github.com/CosmWasm/cosmwasm-template) as a
basis, in particular the `cw-plus` branch.

```bash
cd contracts
cargo generate --git https://github.com/CosmWasm/cosmwasm-template.git --branch cw-plus --name PROJECT_NAME
cd PROJECT_NAME
rm -rf .git
rm .gitignore
rm .cargo-ok
git add .
```

Now, integrate it into the CI and build system

1. Edit `.circleci/config.yml`, copy an existing contracts job and replace the name.
Then add your new job to the jobs list on top. (eg. copy `contracts_cw1_whitelist`
to `contracts_cw721_base` and then replace the 3 instances of `cw1-whitelist` in
that job description with `cw721-base`. And don't forget to add `contracts_cw721_base`
to `workflows.test.jobs`)

1. Add to the `ALL_CONTRACTS` variable in `scripts/publish.sh`

1. Set the `version` variable in `Cargo.toml` to the same version as `packages/cw20`.
For example, "0.5.0" rather than the default "0.1.0" 

1. Edit the root `Cargo.toml` file and add a `profile.release.package.CONTRACT_NAME` 
section, just like `profile.release.package.cw1-subkeys`, but with your
package name.

1. Run `cargo build && cargo test` in the new contract dir

1. Commit all changes and push the branch. Open a PR and ensure the CI runs this.