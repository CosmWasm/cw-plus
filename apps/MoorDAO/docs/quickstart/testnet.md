<!--
order: 3
-->

# Testnet

Learn how to deploy a local testnet or connect to an existing public one {synopsis}

## Pre-requisite Readings

- [Install Aragon](./installation.md) {prereq}
- [Install Docker](https://docs.docker.com/engine/installation/)  {prereq}
- [Install docker-compose](https://docs.docker.com/compose/install/)  {prereq}
<!-- - [Install `jq`](https://stedolan.github.io/jq/download/) {prereq} -->

## Single-node, Local, Manual Testnet

This guide helps you create a single validator node that runs a network locally for testing and other development related uses.

### Initialize node

```bash
$MONIKER=testing
$KEY=mykey
$CHAINID="aragonchain-8"

aragond init $MONIKER --chain-id=$CHAINID
```

::: warning
Monikers can contain only ASCII characters. Using Unicode characters will render your node unreachable.
:::

You can edit this `moniker` later, in the `$(HOME)/.aragond/config/config.toml` file:

```toml
# A custom human readable name for this node
moniker = "<your_custom_moniker>"
```

You can edit the `$HOME/.aragond/config/app.toml` file in order to enable the anti spam mechanism and reject incoming transactions with less than the minimum gas prices:

```toml
# This is a TOML config file.
# For more information, see https://github.com/toml-lang/toml

##### main base config options #####

# The minimum gas prices a validator is willing to accept for processing a
# transaction. A transaction's fees must meet the minimum of any denomination
# specified in this config (e.g. 10ara).

minimum-gas-prices = ""
```

### Genesis Procedure

```bash
# Create a key to hold your account
aragoncli keys add $KEY

# Add that key into the genesis.app_state.accounts array in the genesis file
# NOTE: this command lets you set the number of coins. Make sure this account has some coins
# with the genesis.app_state.staking.params.bond_denom denom, the default is staking
aragond add-genesis-account $(aragoncli keys show validator -a) 1000000000stake,10000000000ara

# Generate the transaction that creates your validator
aragond gentx --name $KEY

# Add the generated bonding transaction to the genesis file
aragond collect-gentxs

# Finally, check the correctness of the genesis.json file
aragond validate-genesis
```

### Run Testnet

Now its safe to start the daemon:

```bash
aragond start
```

You can then stop the node using Ctrl+C.

## Multi-node, Local, Automated Testnet

### Build Testnet & Start Testnet

To build start a 4 node testnet run:

```bash
make localnet-start
```

This command creates a 4-node network using the `aragondnode` Docker image.
The ports for each node are found in this table:

| Node ID       | P2P Port | Tendermint RPC Port | REST/ Ethereum JSON-RPC Port | WebSocket Port |
|---------------|----------|---------------------|------------------------------|----------------|
| `aragonnode0` | `26656`  | `26657`             | `8545`                       | `8546`         |
| `aragonnode1` | `26659`  | `26660`             | `8547`                       | `8548`         |
| `aragonnode2` | `26661`  | `26662`             | `8549`                       | `8550`         |
| `aragonnode3` | `26663`  | `26664`             | `8551`                       | `8552`         |

To update the binary, just rebuild it and restart the nodes

```bash
make localnet-start
```

The command above  command will run containers in the background using Docker compose. You will see the network being created:

```bash
...
Creating network "chainsafe-aragon_localnet" with driver "bridge"
Creating aragondnode0 ... done
Creating aragondnode2 ... done
Creating aragondnode1 ... done
Creating aragondnode3 ... done
```


### Stop Testnet

Once you are done, execute:

```bash
make localnet-stop
```

### Configuration

The `make localnet-start` creates files for a 4-node testnet in `./build` by
calling the `aragond testnet` command. This outputs a handful of files in the
`./build` directory:

```bash
tree -L 3 build/

build/
├── aragoncli
├── aragond
├── gentxs
│   ├── node0.json
│   ├── node1.json
│   ├── node2.json
│   └── node3.json
├── node0
│   ├── aragoncli
│   │   ├── key_seed.json
│   │   └── keyring-test-cosmos
│   └── aragond
│       ├── config
│       ├── data
│       └── aragond.log
├── node1
│   ├── aragoncli
│   │   ├── key_seed.json
│   │   └── keyring-test-cosmos
│   └── aragond
│       ├── config
│       ├── data
│       └── aragond.log
├── node2
│   ├── aragoncli
│   │   ├── key_seed.json
│   │   └── keyring-test-cosmos
│   └── aragond
│       ├── config
│       ├── data
│       └── aragond.log
└── node3
    ├── aragoncli
    │   ├── key_seed.json
    │   └── keyring-test-cosmos
    └── aragond
        ├── config
        ├── data
        └── aragond.log
```

Each `./build/nodeN` directory is mounted to the `/aragond` directory in each container.

### Logging

In order to see the logs of a particular node you can use the following command:

```bash
# node 0: daemon logs
docker exec aragondnode0 tail aragond.log

# node 0: REST & RPC logs
docker exec aragondnode0 tail aragoncli.log
```

The logs for the daemon will look like:

```bash
I[2020-07-29|17:33:52.452] starting ABCI with Tendermint                module=main
E[2020-07-29|17:33:53.394] Can't add peer's address to addrbook         module=p2p err="Cannot add non-routable address 272a247b837653cf068d39efd4c407ffbd9a0e6f@192.168.10.5:26656"
E[2020-07-29|17:33:53.394] Can't add peer's address to addrbook         module=p2p err="Cannot add non-routable address 3e05d3637b7ebf4fc0948bbef01b54d670aa810a@192.168.10.4:26656"
E[2020-07-29|17:33:53.394] Can't add peer's address to addrbook         module=p2p err="Cannot add non-routable address 689f8606ede0b26ad5b79ae244c14cc67ab4efe7@192.168.10.3:26656"
I[2020-07-29|17:33:58.828] Executed block                               module=state height=88 validTxs=0 invalidTxs=0
I[2020-07-29|17:33:58.830] Committed state                              module=state height=88 txs=0 appHash=90CC5FA53CF8B5EC49653A14DA20888AD81C92FCF646F04D501453FD89FCC791
I[2020-07-29|17:34:04.032] Executed block                               module=state height=89 validTxs=0 invalidTxs=0
I[2020-07-29|17:34:04.034] Committed state                              module=state height=89 txs=0 appHash=0B54C4DB1A0DACB1EEDCD662B221C048C826D309FD2A2F31FF26BAE8D2D7D8D7
I[2020-07-29|17:34:09.381] Executed block                               module=state height=90 validTxs=0 invalidTxs=0
I[2020-07-29|17:34:09.383] Committed state                              module=state height=90 txs=0 appHash=75FD1EE834F0669D5E717C812F36B21D5F20B3CCBB45E8B8D415CB9C4513DE51
I[2020-07-29|17:34:14.700] Executed block                               module=state height=91 validTxs=0 invalidTxs=0
```

::: tip
You can disregard the `Can't add peer's address to addrbook` warning. As long as the blocks are
being produced and the app hashes are the same for each node, there should not be any issues.
:::

Whereas the logs for the REST & RPC server would look like:

```bash
I[2020-07-30|09:39:17.488] Starting application REST service (chain-id: "aragonchain-8")... module=rest-server
I[2020-07-30|09:39:17.488] Starting RPC HTTP server on 127.0.0.1:8545   module=rest-server
...
```

#### Follow Logs

You can also watch logs as they are produced via Docker with the `--follow` (`-f`) flag, for
example:

```bash
docker logs -f aragondnode0
```

### Interact With the Testnet

#### Ethereum JSON RPC & Websocket Ports

To interact with the testnet via WebSockets or RPC/API, you will send your request to the corresponding ports:

| Eth JSON-RPC | Eth WS |
|--------------|--------|
| `8545`       | `8546` |

You can send a curl command such as:

```bash
curl -X POST --data '{"jsonrpc":"2.0","method":"eth_accounts","params":[],"id":1}' -H "Content-Type: application/json" 192.162.10.1:8545
```

::: tip
The IP address will be the public IP of the docker container.
:::

Additional instructions on how to interact with the WebSocket can be found on the [events documentation](./events.md#ethereum-websocket).

### Keys & Accounts

To interact with `aragoncli` and start querying state or creating txs, you use the
`aragoncli` directory of any given node as your `home`, for example:

```bash
aragoncli keys list --home ./build/node0/aragoncli
```

Now that accounts exists, you may create new accounts and send those accounts
funds!

::: tip
**Note**: Each node's seed is located at `./build/nodeN/aragoncli/key_seed.json` and can be restored to the CLI using the `aragoncli keys add --restore` command
:::

### Special Binaries

If you have multiple binaries with different names, you can specify which one to run with the BINARY environment variable. The path of the binary is relative to the attached volume. For example:

```bash
# Run with custom binary
BINARY=aragon make localnet-start
```

## Genesis and Seeds

### Copy the Genesis File

<!-- TODO: link to genesis procedure -->
::: tip
If you want to start a network from scratch, you will need to start the genesis procedure.
:::

If you want to connect to an existing testnet, fetch the testnet's `genesis.json` file and copy it into the `aragonchaind`'s config directory (i.e `$HOME/.aragonchaind/config/genesis.json`).

Then verify the correctness of the genesis configuration file:

```bash
aragonchaind validate-genesis
```

### Add Seed Nodes

Your node needs to know how to find peers. You'll need to add healthy seed nodes to `$HOME/.aragonchaind/config/config.toml`. If those seeds aren't working, you can find more seeds and persistent peers on an existing explorer.

For more information on seeds and peers, you can the Tendermint [P2P documentation](https://docs.tendermint.com/master/spec/p2p/peer.html).

### Start testnet

The final step is to [start the nodes](./run_node.md#start-node). Once enough voting power (+2/3) from the genesis validators is up-and-running, the testnet will start producing blocks.

## Testnet faucet

Once the aragon-chain daemon is up and running, you can request tokens to your address using the `faucet` module:

```bash
# query your initial balance
aragonchaincli q bank balances $(aragonchaincli keys show <mykey> -a)  

# send a tx to request tokens to your account address
aragonchaincli tx faucet request 100ara --from <mykey>

# query your balance after the request
aragonchaincli q bank balances $(aragonchaincli keys show <mykey> -a)
```

You can also check to total amount funded by the faucet and the total supply of the chain via:

```bash
# total amount funded by the faucet
aragonchaincli q faucet funded

# total supply
aragonchaincli q supply total
```

## Next {hide}

Learn about how to setup a [validator](./validator-setup.md) node on Aragon Chain {hide}
