<!--
order: 1
-->

# Installation

## Binaries

Clone and build Aragon Chain using `git`:

```bash
git clone https://github.com/aragon/aragon-chain.git
cd aragon-chain
make install
```

Check that the binaries have been successfuly installed:

```bash
aragonchaind -h
aragonchaincli -h
```

<!-- ## Docker -->

<!-- TODO: -->

## Releases

::: warning
Aragon Chain is under VERY ACTIVE DEVELOPMENT and should be treated as pre-alpha software. This means it is not meant to be run in production, its APIs are subject to change without warning and should not be relied upon, and it should not be used to hold any value. We will remove this warning when we have a release that is stable, secure, and properly tested.
:::

You can also download a specific release available on the [Aragon repository](https://github.com/aragon/aragon-chain/releases)

## Next {hide}

Learn how to [run a node](./.run_node.md) {hide}
