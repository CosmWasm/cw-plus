#!/bin/bash
set -o errexit -o nounset -o pipefail
command -v shellcheck > /dev/null && shellcheck "$0"

# these are imported by other packages
BASE_PACKAGES="cw0 storage-plus"
ALL_PACKAGES="cw1 cw2 cw3 cw4 cw20 cw721"

# these are imported by other contracts
BASE_CONTRACTS="cw1-whitelist cw4-group cw20-base cw721-base"
ALL_CONTRACTS="cw1-subkeys cw3-fixed-multisig cw3-flex-multisig cw20-atomic-swap cw20-escrow cw20-staking"

SLEEP_TIME=30

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
