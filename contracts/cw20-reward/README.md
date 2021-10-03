# Anchor bAsset Reward  <!-- omit in toc -->

**NOTE**: Reference documentation for this contract is available [here](https://anchor-protocol.gitbook.io/anchor/bluna/reward).

The Reward contract contains logic for distributing Luna delegation rewards to holders of bLuna. After the Hub contract withdraws Luna delegation rewards to the Reward contract, the Hub contract can request all reward to a single denomination (Terra USD), which can then be distributed to bLuna holders. Holders of bLuna can then send a request to this contract to claim their accrued rewards.
The Reward contract also stores the balance and reward index values for all bLuna holders, which is used to calculate the amount of bLuna rewards that a specific holder has accrued.
