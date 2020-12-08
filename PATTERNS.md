# Contract Calling Patterns

CosmWasm follows the actor model, but wraps it in atomic transactions.
The standard contract call pattern is to simply return messages that must
succeed, and the contract aborts if they don't. This covers > 90% of the
cases we come across of, such as transferring tokens (native or cw20) or
staking, etc based upon some contract state transition.

We can query to predict success, or store a local counter of available
balance, if we want, but in the end, we optimistically
commit changes as if all other actions succeed, and just fire and forget,
trusting the system to roll it back if they fail.

This means there are 3 cases we do not handle:

1. If you want to get the return value of a message call and do some
    processing based on it.
2. If you want to handle error cases with something else than aborting
    the transaction. (This includes preventing an dispatched message from 
    consuming all gas and aborting the original message).
3. Listening to events/notifications of state changes of other contracts

Here we will describe some patterns to let you cover some (but not all)
of these cases.

## Callbacks

This is the most common case people ask for, and can easily be handled
with some clever use of the existing system.

The first case is where we want to do action A, then finish our processing.
A clear example is in `cw20-staking`, where we want to withdraw all rewards
from the validator, then reinvest them, as one atomic operation.
In this case, we simply [return 2 messages](https://github.com/CosmWasm/cosmwasm-plus/blob/master/contracts/cw20-staking/src/contract.rs#L383-L395),
the first one calling the staking module, and the second one calling a 
protected function on our own contract `_BondAllTokens`.
At the beginning of `_BondAllTokens`, we ensure this is 
[called by our own contract](https://github.com/CosmWasm/cosmwasm-plus/blob/master/contracts/cw20-staking/src/contract.rs#L408-L410)
to keep this a private callback and not a public entry point.

The second case is where we want to get a result from the call, such as
the contract address of a new contract. We need support from the called 
contract to be able to do this. On the message, we pass an (optional)
"callback" field with the address of the contract to inform. The called
contract will then send a message in a pre-defined format to that contract.

Example:

```rust
// called contract
pub struct InitMsg {
    pub some_data: String,
    pub callback: Option<HumanAddr>,
}

// dispatch (like cw20 send), the callback contract
// must support a superset of this in HandleMsg
pub enum CallbackMsg {
    Instantiated{
        contract: HumanAddr,
    }
}

// init inside the called contract
pub fn init(deps: DepsMut,
            env: Env,
            info: MessageInfo,
            msg: InitMsg) {
    // do your main init logic....
    
    // create a callback message if needed
    let mut messages: Vec<CosmosMsg> = vec![];
    if let Some(callback) = msg.callback {
        let data = &CallbackMsg::Instantiated { 
            contract: env.contract.address
        };
        let msg = to_binary(data)?;
        let wasm = WasmMsg::Execute {
            contract_addr: callback,
            msg,
            send: vec![],
        };
        messages.push(wasm.into())
    }
    
    Ok(HandleResponse{
        messages,
    })
}
```

## Isolating contracts

We don't currently support any technique for dispatching a message that is 
not atomically executing in the same context (and using the same gas) as
the calling contract. 

We are investigating this in the context of building
a cron service. (TODO: link to CyberCongress design).
When there is an approach that allows this, it will be documented here.
It will need a custom native (Cosmos SDK) module to enable it.

## Subscribing to Events

There are two types of event subscriptions. Both require that the contract
emitting the events supports this explicitly. They are "hooks" and "listeners".
The main difference is that hooks are executed synchronously and atomic in the
same context as the contract (and can abort the original call on error),
while listeners are executed asynchronously in their own context, only
*after the original state change has been committed*.

### Hooks

Hooks are the simplest to implement and most powerful. You have a handle
method to add/remove hooks to the contract, and everytime a particular state
change happens, a predefined message is sent to all hooks. If any of these
fail (or it runs out of gas), the transaction is aborted.

This is powerful, meaning that "hook" can do some arbitrary checks and
abort the original change if it disapproves. For example, a multisig
could decide the group cannot change when a proposal is open. If a 
"change membership" hook is executed, it can check if there are any
open proposals, and if so, return an error.

The downside, is that you must trust the hooked contracts highly. They
cannot break the logic of the original contract, but they can "brick it"
(Denial of Service - when they return an error and the datastore transaction gets rolled back). They also increase the gas cost of the normal call,
and must be kept to a very limited set, or the gas price will be extremely
high. Currently, I would only support this if you need an "admin"
(or InitMsg) to set/remove hooks. And removing hooks does itself not trigger
a hook, so this can always succeed to remove load if the hooks start
making trouble.

To see an example of hooks, please check out how `cw3-flex-multisig`
registers on `cw4-group` to be informed of any changes to the voting set.
This is essential to manage proper vote counts when the voting set changes
while a proposal is open.

* [Definition of the hooks in cw4 spec](https://github.com/CosmWasm/cosmwasm-plus/blob/c5e8fc92c0412fecd6cdd951c2c0261aa3c9445a/packages/cw4/src/hook.rs)
* [Adding/removing hooks](https://github.com/CosmWasm/cosmwasm-plus/blob/11400ddcc18d56961b0592a655e3da9cba7fd5d8/contracts/cw4-group/src/contract.rs#L156-L190) - which may be refactored into common code
* [Dispatching updates to all registered hooks](https://github.com/CosmWasm/cosmwasm-plus/blob/11400ddcc18d56961b0592a655e3da9cba7fd5d8/contracts/cw4-group/src/contract.rs#L91-L98)
* [`cw3-flex-multisig` registers HandleMsg variant](https://github.com/CosmWasm/cosmwasm-plus/blob/db560558c901a2bda933d035dbbc30321c3c66ff/contracts/cw3-flex-multisig/src/msg.rs#L38-L39)
* [`cw3-flex-multisig` updates state based on the hook](https://github.com/CosmWasm/cosmwasm-plus/blob/61f436c2203bde7770d9b13724e6548ba26615e7/contracts/cw3-flex-multisig/src/contract.rs#L276-L309)

### Listeners

There is currently no clean way to support listeners, as that would require
some way of isolating contracts as a prerequisite. We can add some thoughts
on how this will look once that exists when it is possible.

In theory, we want to act like hooks, but executed in a different context.

TODO: when this is possible
