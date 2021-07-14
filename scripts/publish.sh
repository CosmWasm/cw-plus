#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck > /dev/null && shellcheck "$0"

# this should really more to cosmwasm...
STORAGE_PACKAGES="storage-plus"
# these are imported by other packages
BASE_PACKAGES="cw0"
ALL_PACKAGES="controllers cw1 cw2 cw3 cw4 cw20 cw721 cw1155 multi-test"

# these are imported by other contracts
BASE_CONTRACTS="cw1-whitelist  cw4-group cw20-base cw721-base"
ALL_CONTRACTS="cw1-subkeys cw3-fixed-multisig cw3-flex-multisig cw4-stake cw20-atomic-swap cw20-bonding cw20-escrow cw20-ics20 cw20-staking cw1155-base"

SLEEP_TIME=30

for pack in $STORAGE_PACKAGES; do
  (
    cd "packages/$pack"
    echo "Publishing $pack"
    cargo publish
  )
done

# wait for these to be processed on crates.io
echo "Waiting for publishing storage packages"
sleep $SLEEP_TIME

for pack in $BASE_PACKAGES; do
  (
    cd "packages/$pack"
    echo "Publishing $pack"
    cargo publish
  )
done

# wait for these to be processed on crates.io
echo "Waiting for publishing base packages"
sleep $SLEEP_TIME

for pack in $ALL_PACKAGES; do
  (
    cd "packages/$pack"
    echo "Publishing $pack"
    cargo publish
  )
done

# wait for these to be processed on crates.io
echo "Waiting for publishing all packages"
sleep $SLEEP_TIME

for cont in $BASE_CONTRACTS; do
  (
    cd "contracts/$cont"
    echo "Publishing $cont"
    cargo publish
  )
done

# wait for these to be processed on crates.io
echo "Waiting for publishing base packages"
sleep $SLEEP_TIME

for cont in $ALL_CONTRACTS; do
  (
    cd "contracts/$cont"
    echo "Publishing $cont"
    cargo publish
  )
done

echo "Everything is published!"
