# Contract Calling Patterns

CosmWasm follows the actor model, but wraps it in atomic transactions.
The standard contract call pattern is to simply return messages that must
succeed, and the contract aborts if they don't. This covers > 90% of the
cases we come across of, such as transfering tokens (native or cw20) or
staking, etc based upon some contract state transition.

We can query to predict success, or store a local counter of available
balance, if we want, but in the end, we optimistically
commit changes as if all other actions succeed, and just fire and forget,
trusting the system to roll it back if they fail.

This means there are 3 places we do not handle:

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