## A Go Implementation of the Polkadot Host

Gossamer is an implementation of the [Polkadot Host](https://github.com/w3f/polkadot-spec): a framework used to build and run nodes for different blockchain protocols that are compatible with the Polkadot ecosystem.  The core of the Polkadot Host is the wasm runtime which handles the logic of the chain.

Gossamer includes node implementations for major blockchains within the Polkadot ecosystem and simplifies building node implementations for other blockchains. Runtimes built with [Substrate](https://github.com/paritytech/substrate) can plug their runtime into Gossamer to create a node implementation in Go.

For more information about Gossamer, the Polkadot ecosystem, and how to use Gossamer to build and run nodes for various blockchain protocols within the Polkadot ecosystem, check out the [Gossamer Docs](https://ChainSafe.github.io/gossamer).

## Get Started

### Prerequisites

install go version `>=1.14`

### Installation

get the [ChainSafe/gossamer](https://github.com/ChainSafe/gossamer) repository:
```
go get -u github.com/ChainSafe/gossamer
```

You may encounter a `package github.com/ChainSafe/gossamer: no Go files in ...` message. This is not an error, since there are no go files in the project root. 

build gossamer command:
```
make gossamer
```

### Run Default Node

initialize default node:
```
./bin/gossamer init
```

start default node:
```
./bin/gossamer --key alice
```

The built-in keys available for the node are `alice`, `bob`, `charlie`, `dave`, `eve`, `ferdie`, `george`, and `ian`.

### Run Gossamer Node

initialize gossamer node:
```
./bin/gossamer --chain gssmr init
```

start gossamer node:
```
./bin/gossamer --chain gssmr --key alice
```

### Run Kusama Node (_in development_)

initialize kusama node:
```
./bin/gossamer --chain ksmcc --key alice init
```

start kusama node:
```
./bin/gossamer --chain ksmcc --key alice
```

### Run Polkadot Node (_in development_)

initialize polkadot node:
```
./bin/gossamer --chain dotcc --key alice init
```

start polkadot node:
```
./bin/gossamer --chain dotcc --key alice
```

## Contribute

- Check out [Contributing Guidelines](.github/CONTRIBUTING.md)  
- Have questions? Say hi on [Discord](https://discord.gg/Xdc5xjE)!

## Donate

Our work on gossamer is funded by grants. If you'd like to donate, you can send us ETH or DAI at the following address:
`0x764001D60E69f0C3D0b41B0588866cFaE796972c`

## ChainSafe Security Policy

### Reporting a Security Bug

We take all security issues seriously, if you believe you have found a security issue within a ChainSafe
project please notify us immediately. If an issue is confirmed, we will take all necessary precautions 
to ensure a statement and patch release is made in a timely manner.

Please email us a description of the flaw and any related information (e.g. reproduction steps, version) to
[security at chainsafe dot io](mailto:security@chainsafe.io).


## License

_GNU Lesser General Public License v3.0_

<br />
<p align="center">
	<img src="/docs/assets/img/chainsafe_gopher.png">
</p>

