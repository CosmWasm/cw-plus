#!/bin/bash

KEY="mykey"
CHAINID="aragonchain-1123698127639817236"
MONIKER="localtestnet"

# remove existing daemon and client
rm -rf ~/.aragonchain*

make install

aragonchaincli config keyring-backend test

# Set up config for CLI
aragonchaincli config chain-id $CHAINID
aragonchaincli config output json
aragonchaincli config indent true
aragonchaincli config trust-node true

# if $KEY exists it should be deleted
aragonchaincli keys add $KEY

# Set moniker and chain-id for Aragon (Moniker can be anything, chain-id must be an integer)
aragonchaind init $MONIKER --chain-id $CHAINID

cat $HOME/.aragonchaind/config/genesis.json | jq '.app_state["staking"]["params"]["bond_denom"]="ara"' > $HOME/.aragonchaind/config/tmp_genesis.json && mv $HOME/.aragonchaind/config/tmp_genesis.json $HOME/.aragonchaind/config/genesis.json
cat $HOME/.aragonchaind/config/genesis.json | jq '.app_state["crisis"]["constant_fee"]["denom"]="ara"' > $HOME/.aragonchaind/config/tmp_genesis.json && mv $HOME/.aragonchaind/config/tmp_genesis.json $HOME/.aragonchaind/config/genesis.json
cat $HOME/.aragonchaind/config/genesis.json | jq '.app_state["gov"]["deposit_params"]["min_deposit"][0]["denom"]="ara"' > $HOME/.aragonchaind/config/tmp_genesis.json && mv $HOME/.aragonchaind/config/tmp_genesis.json $HOME/.aragonchaind/config/genesis.json
cat $HOME/.aragonchaind/config/genesis.json | jq '.app_state["mint"]["params"]["mint_denom"]="ara"' > $HOME/.aragonchaind/config/tmp_genesis.json && mv $HOME/.aragonchaind/config/tmp_genesis.json $HOME/.aragonchaind/config/genesis.json
cat $HOME/.aragonchaind/config/genesis.json | jq '.app_state["faucet"]["enable_faucet"]=true' > $HOME/.aragonchaind/config/tmp_genesis.json && mv $HOME/.aragonchaind/config/tmp_genesis.json $HOME/.aragonchaind/config/genesis.json
cat $HOME/.aragonchaind/config/genesis.json | jq '.app_state["evm"]["params"]["evm_denom"]="ara"' > $HOME/.aragonchaind/config/tmp_genesis.json && mv $HOME/.aragonchaind/config/tmp_genesis.json $HOME/.aragonchaind/config/genesis.json

# Allocate genesis accounts (cosmos formatted addresses)
aragonchaind add-genesis-account $(aragonchaincli keys show mykey -a) 1000000000000000000ara

# Sign genesis transaction
aragonchaind gentx --amount=100000000ara --name $KEY --keyring-backend test

# Collect genesis tx
aragonchaind collect-gentxs

echo -e '\n\ntestnet faucet enabled'
echo -e 'to transfer tokens to your account address use:'
echo -e "aragonchaincli tx faucet request 100ara --from $KEY\n"

# Run this to ensure everything worked and that the genesis file is setup correctly
aragonchaind validate-genesis

# Command to run the rest server in a different terminal/window
echo -e '\nrun the following command in a different terminal/window to run the REST server and JSON-RPC:'
echo -e "aragonchaincli rest-server --laddr \"tcp://localhost:8545\" --unlock-key $KEY --chain-id $CHAINID --trace\n"

# Start the node (remove the --pruning=nothing flag if historical queries are not needed)
aragonchaind start --pruning=nothing --rpc.unsafe --log_level "main:info,state:info,mempool:info" --trace
