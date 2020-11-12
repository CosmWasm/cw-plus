# Substrate IBC Pallet

## Purpose

This pallet implements the standard [IBC protocol](https://github.com/cosmos/ics).

The goal of this pallet is to allow the blockchains built on Substrate to gain the ability to interact with other chains in a trustless way via IBC protocol, no matter what consensus the counterparty chains use. Some components in [ICS spec](https://github.com/cosmos/ics/tree/master/spec) are implemented to support a working demo (https://github.com/cdot-network/ibc-demo), but not fully implemented as the spec:  


Here is a [demo](https://github.com/cdot-network/ibc-demo) for showing how to utilize this pallet, which initializes a series of steps for cross-chain communication, from client creation to sending packet data.

## Dependencies

### Traits

This pallet does not depend on any externally defined traits.

### Pallets

This pallet does not depend on any other FRAME pallet or externally developed modules.

## Installation

### Runtime `Cargo.toml`

To add this pallet to your runtime, simply include the following to your runtime's `Cargo.toml` file:

```TOML
[dependencies.pallet-ibc]
default_features = false
git = 'https://github.com/cdot-network/substrate-ibc.git'
```

and update your runtime's `std` feature to include this pallet:

```TOML
std = [
    # --snip--
        'pallet-ibc/std',
	]
```

### Runtime `lib.rs`
A custom structure that implements the pallet_ibc::ModuleCallbacks must be defined to dispatch messages to receiving module.
```rust
pub struct ModuleCallbacksImpl;

impl pallet_ibc::ModuleCallbacks for ModuleCallbacksImpl {
    # --snip--
}
```

You should implement it's trait like so:

```rust
/// Used for test_module
impl pallet_ibc::Trait for Runtime {
	type Event = Event;
	type ModuleCallbacks = ModuleCallbacksImpl;
}
```

and include it in your `construct_runtime!` macro:

```rust
Ibc: pallet_ibc::{Module, Call, Storage, Event<T>},
```

### Genesis Configuration

This pallet does not have any genesis configuration.

## How to Interact with the Pallet
### At Runtime
In the ibc-demo repo, substrate-subxt invokes the pallet's callable functions by the macro ```substrate_subxt_proc_macro::Call```.

Let's take the function ```test_create_client``` as an example. [Client](https://docs.rs/substrate-subxt/0.12.0/substrate_subxt/struct.Client.html) extends the function 
```rust
// in https://github.com/cdot-network/ibc-demo/blob/master/pallets/template/src/lib.rs
pub fn test_create_client(
    origin,
    identifier: H256,
    height: u32,
    set_id: SetId,
    authorities: AuthorityList,
    root: H256
) -> dispatch::DispatchResult {
...
}
``` 
by 
```rust
// https://github.com/cdot-network/ibc-demo/blob/master/calls/src/template.rs
#[derive(Encode, Call)]
pub struct TestCreateClientCall<T: TemplateModule> {
    pub _runtime: PhantomData<T>,
    pub identifier: H256,
    pub height: u32,
    pub set_id: SetId,
    pub authority_list: AuthorityList,
    pub root: H256,
}
```

Therefore, 
```rust
//  https://github.com/cdot-network/ibc-demo/blob/master/cli/src/main.rs
client
.test_create_client(...)
```
can invoke the ```test_create_client``` function. 


### At Unit Test
In unit test, we comply with the substrate's document [Runtime Tests](https://substrate.dev/docs/en/knowledgebase/runtime/tests). 

The mock enviroment is built in [mock.rs](src/mock.rs); In [tests.rs](src/tests.rs), the pallet's callable functions are tested.

## Implementation Logic in Source Code

### Synchronizing Block Headers of Other Chains
* Relayers send latest block headers of other chains to ibc pallet by invoking the ```Datagram::ClientUpdate``` arm:
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        Datagram::ClientUpdate { identifier, header } => {   // <--- "Datagram::ClientUpdate" will be matached
```
* If verified, the incoming block header's commitment_root and block height is inserted to storage ```ConsensusStates```.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
ConsensusStates::insert((identifier, header.height), new_consensus_state);
```  

### Connection Opening Handshakes - ICS-003
As the table in [Opening Handshake](https://github.com/cosmos/ics/tree/master/spec/ics-003-connection-semantics#opening-handshake), the handshakes between 2 chains(A & B) comprises 4 steps.

| Initiator | Datagram          | Chain acted upon | Prior state (A, B) | Posterior state (A, B) |
| --------- | ----------------- | ---------------- | ------------------ | ---------------------- |
| Actor     | `ConnOpenInit`    | A                | (none, none)       | (INIT, none)           |
| Relayer   | `ConnOpenTry`     | B                | (INIT, none)       | (INIT, TRYOPEN)        |
| Relayer   | `ConnOpenAck`     | A                | (INIT, TRYOPEN)    | (OPEN, TRYOPEN)        |
| Relayer   | `ConnOpenConfirm` | B                | (OPEN, TRYOPEN)    | (OPEN, OPEN)           |

#### (none, none) -> (INIT, none)
It's done by an actor, who invokes the function ```conn_open_init``` in Chain A.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn conn_open_init(
    identifier: H256,
    desired_counterparty_connection_identifier: H256,
    client_identifier: H256,
    counterparty_client_identifier: H256,
) -> dispatch::DispatchResult {
...
}
```

#### (INIT, none) -> (INIT, TRYOPEN)
The relayer detects the ```INIT``` state of chain A's connection, then try to set chain B's connection's state to ```TRYOPEN``` by invoking the chain B's function ```pub fn handle_datagram(datagram: Datagram)```, 
whose arm ```Datagram::ConnOpenTry``` will be matached.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::ConnOpenTry {
            ...
        }
```

#### (INIT, TRYOPEN) -> (OPEN, TRYOPEN)
The relayer detects the ```TRYOPEN``` of chain B's connection, then try to set chain A's connection's state to ```OPEN``` by invoking the chain A's function ```pub fn handle_datagram(datagram: Datagram)```, 
whose arm ```Datagram::ConnOpenAck``` will be matached.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::ConnOpenAck {
            ...
        }
```

#### (OPEN, TRYOPEN) -> (OPEN, OPEN)
The relayer detects the ```OPEN``` of chain A's connection, then try to set chain B's connection's state to ```OPEN``` by invoking the chain B's function ```pub fn handle_datagram(datagram: Datagram)```, 
whose arm ```Datagram::ConnOpenConfirm``` will be matached.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::ConnOpenConfirm {
            ...
        }
```

### Channel Opening Handshakes - ICS-004
After the 2 chains(A & B) finish connection handshakes, they are able to build a channel by handshakes on the connection.

As the table in [Channel lifecycle management](https://github.com/cosmos/ics/tree/master/spec/ics-004-channel-and-packet-semantics#channel-lifecycle-management), the handshakes between 2 chains(A & B) comprises 4 steps.

| Initiator | Datagram         | Chain acted upon | Prior state (A, B) | Posterior state (A, B) |
| --------- | ---------------- | ---------------- | ------------------ | ---------------------- |
| Actor     | ChanOpenInit     | A                | (none, none)       | (INIT, none)           |
| Relayer   | ChanOpenTry      | B                | (INIT, none)       | (INIT, TRYOPEN)        |
| Relayer   | ChanOpenAck      | A                | (INIT, TRYOPEN)    | (OPEN, TRYOPEN)        |
| Relayer   | ChanOpenConfirm  | B                | (OPEN, TRYOPEN)    | (OPEN, OPEN)           |

#### (none, none) -> (INIT, none)
It's done by an actor, who invokes the function ```chan_open_init``` in Chain A.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn chan_open_init(
    ...
) -> dispatch::DispatchResult {
...
}
```

#### (INIT, none) -> (INIT, TRYOPEN)
The relayer detects the ```INIT``` state of chain A's channel, then try to set chain B's channel's state to ```TRYOPEN``` by invoking the chain B's function ```pub fn handle_datagram(datagram: Datagram)```, 
whose arm ```Datagram::ChanOpenTry``` will be matached.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::ChanOpenTry {
            ...
        }
```

#### (INIT, TRYOPEN) -> (OPEN, TRYOPEN)
The relayer detects the ```TRYOPEN``` of chain B's channel, then try to set chain A's channel's state to ```OPEN``` by invoking the chain A's function ```pub fn handle_datagram(datagram: Datagram)```, 
whose arm ```Datagram::ChanOpenAck``` will be matached.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::ChanOpenAck {
            ...
        }
```

#### (OPEN, TRYOPEN) -> (OPEN, OPEN)
The relayer detects the ```OPEN``` of chain A's channel, then try to set chain B's channel's state to ```OPEN``` by invoking the chain B's function ```pub fn handle_datagram(datagram: Datagram)```, 
whose arm ```Datagram::ChanOpenConfirm``` will be matached.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::ChanOpenConfirm {
            ...
        }
```

### Packet Flow & Handling - ICS-004
After the 2 chains(A & B) finish channel handshakes, they are able to send packets to each other on the channel.

As the flowchart in [Packet flow & handling](https://github.com/cosmos/ics/tree/master/spec/ics-004-channel-and-packet-semantics#packet-flow--handling), the standard flow of sending a packet from chain A to chain B comprises 3 steps.

#### Sending a Packet
The callable function ```send_packet``` in Chain A's ibc pallet sends a packet by depositing an ```RawEvent::SendPacket``` event.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn send_packet(packet: Packet) -> dispatch::DispatchResult {
    ...
    Self::deposit_event(RawEvent::SendPacket(
        ...
    ));
}
```

#### Receiving a Packet and Writing an Acknowledgement
The relayer detects chain A's ```RawEvent::SendPacket``` event, then try to call chain B's function ```pub fn handle_datagram(datagram: Datagram)```, 
and match its arm ```Datagram::PacketRecv```, for chain B to receive the packet.

After receiving the packet, chain B deposits an event ```RawEvent::RecvPacket``` as acknowledgement.

```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::PacketRecv {
            ...

            Self::deposit_event(RawEvent::RecvPacket(
                ...
            ));
        }
```

#### Processing an Acknowledgement
The relayer detects chain B's ```RawEvent::RecvPacket``` event, then try to call chain A's function ```pub fn handle_datagram(datagram: Datagram)```, 
and match its arm ```Datagram::PacketAcknowledgement```, for chain A to process the acknowledgement.
```rust
// https://github.com/cdot-network/substrate-ibc/blob/master/src/lib.rs
pub fn handle_datagram(datagram: Datagram) -> dispatch::DispatchResult {
    match datagram {
        ...
        Datagram::PacketAcknowledgement {
            ...
        }
```

## Reference Docs

You can view the reference docs for this pallet by running:

```
cargo doc --open
```

or by visiting this site: <Add Your Link>

