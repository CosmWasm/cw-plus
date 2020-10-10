# Substrate light client
Implementation of substrate light client in rust compilable to wasm. It is written to be integrated with CosmWasm readily, and is optimized to run in a constrained environment of a smart contract. Refer [here](#how-it-works) to know how it works under the hood.

## Compilation

### Prerequisites
1. Rust 1.42.0 or higher
2. Two target need to be installed
    1. `wasm32-unknown-unknown` to compile it into wasm and integrate it with CosmWasm
    2. `x86_64-apple-darwin` to run tests

### Compile in wasm
Run `make wasm` in project directory. This will produce a file `/target/wasm32-unknown-unknown/release/substrate_client.wasm`
To produce a size optimized build, you need to run `make wasm-optimized`.

### Testing
1. Run all the tests:
`cargo test`
2. Run the test tool:
Test tool is a bash script that run two tests with `-- --nocapture` flag, which makes them print out execution trace.
```commandline
chmod +x test-tool.sh
./test-tool.sh
```

## Run it inside Cosmos blockchain
Before we start, we need to build wasm optimized byte code for this light client via running `make wasm-optimized`.

To run it inside Cosmos blockchain, we need modified version of `Gaia` with CosmWasm enabled. For that, you can clone this [repository](https://github.com/ChorusOne/gaia), and switch to `wasm-ibc` branch. Then follow instruction from README file to upload it to gaia daemon and start gaia LCD.

## How it works?
At a higher level, substrate light client follows the sequence of grandpa finalized headers and keeps track of the following things:
1. Best header seen till now: Refers to the last header we successfully ingested.
2. Last finalized header: Last header for which we received a valid grandpa justification
3. Scheduled Grandpa Authority Set Change: It refers to the change of authority set after a delay of certain blocks. It is extracted from `ScheduledChange` consensus log from the incoming header and kept in the storage till the authority set change is applied to the current authority set.
4. Current Grandpa Authority set: Grandpa authority set after last authority set change was applied. It is used to validate grandpa justification.

Light client is in form of CosmWasm contract, with three entry points: 
1. `init`: As the name suggests, init method initializes new light client instance. It requires a root header and grandpa authority set who signed that header along with some configuration parameters.
2. `update`: update method ingests incoming header with optional justification. Header ingestion first validates incoming header (optionally with justification), and contains mainly two checks: a. Header is a child of the last header we successfully ingested b. If justification is provided, it is valid against current authority set and its target hash is equal to header's hash. Upon successful validation, if a scheduled authority set change is contained in the header, it is extracted and stored along with the header. Lastly, if valid justification is provided, the header and its ascendants are marked as finalized.
3. `query`: Query method is a read-only method that reads light client storage and returns data like last ingested header, last finalized header, etc.
