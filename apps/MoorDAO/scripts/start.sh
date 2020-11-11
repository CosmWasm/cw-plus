#!/bin/sh
aragonchaind --home /aragonchain/node$ID/aragonchaind/ start > aragonchaind.log &
sleep 5
aragonchaincli rest-server --laddr "tcp://localhost:8545" --chain-id "aragonchain-7305661614933169792" --trace > aragonchaincli.log &
tail -f /dev/null