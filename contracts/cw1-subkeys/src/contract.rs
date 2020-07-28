use schemars::JsonSchema;
use std::fmt;

use cosmwasm_std::{
    log, to_binary, Api, Binary, CosmosMsg, Empty, Env, Extern, HandleResponse, InitResponse,
    Querier, StdResult, Storage,
};
use cw1_whitelist::{
    contract::{handle_freeze, handle_update_admins, init as whitelist_init, query_config},
    msg::InitMsg,
    state::config_read,
};

use crate::msg::{HandleMsg, QueryMsg};
use crate::state::{allowances, allowances_read, Allowance};

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> StdResult<InitResponse> {
    whitelist_init(deps, env, msg)
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: HandleMsg<Empty>,
) -> StdResult<HandleResponse<Empty>> {
    match msg {
        HandleMsg::Execute { msgs } => handle_execute(deps, env, msgs),
        HandleMsg::Freeze {} => handle_freeze(deps, env),
        HandleMsg::UpdateAdmins { admins } => handle_update_admins(deps, env, admins),
        HandleMsg::IncreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_increase_allowance(deps, env, spender, amount, expires),
        HandleMsg::DecreaseAllowance {
            spender,
            amount,
            expires,
        } => handle_decrease_allowance(deps, env, spender, amount, expires),
    }
}

pub fn handle_execute<S: Storage, A: Api, Q: Querier, T>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msgs: Vec<CosmosMsg<T>>,
) -> StdResult<HandleResponse<T>>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    let cfg = config_read(&deps.storage).load()?;
    // this is the admin behavior (same as cw1-whitelist)
    if cfg.is_admin(&deps.api.canonical_address(&env.message.sender)?) {
        let mut res = HandleResponse::default();
        res.messages = msgs;
        res.log = vec![log("action", "execute")];
        Ok(res)
    } else {
        // TODO
        // for each message, check if the message is a BankMsg::Send and the if the subkey has sufficient allowance.
        // if so, reduce the allowance and resend this message
        //
        // Note, you may want to use the cosmwasm_std::Context object to build the response
        panic!("unimplemented")
    }
}

pub fn query<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    msg: QueryMsg,
) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::Allowance { spender } => to_binary(&query_allowance(deps, spender)?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coins, BankMsg, StdError, WasmMsg};

    const CANONICAL_LENGTH: usize = 20;

    #[test]
    fn init_and_modify_config() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let carl = HumanAddr::from("carl");

        let anyone = HumanAddr::from("anyone");

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            mutable: true,
        };
        let env = mock_env(&deps.api, &anyone, &[]);
        init(&mut deps, env, init_msg).unwrap();

        // ensure expected config
        let expected = ConfigResponse {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            mutable: true,
        };
        assert_eq!(query_config(&deps).unwrap(), expected);

        // anyone cannot modify the contract
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![anyone.clone()],
        };
        let env = mock_env(&deps.api, &anyone, &[]);
        let res = handle(&mut deps, env, msg);
        match res.unwrap_err() {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but alice can kick out carl
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![alice.clone(), bob.clone()],
        };
        let env = mock_env(&deps.api, &alice, &[]);
        handle(&mut deps, env, msg).unwrap();

        // ensure expected config
        let expected = ConfigResponse {
            admins: vec![alice.clone(), bob.clone()],
            mutable: true,
        };
        assert_eq!(query_config(&deps).unwrap(), expected);

        // carl cannot freeze it
        let env = mock_env(&deps.api, &carl, &[]);
        let res = handle(&mut deps, env, HandleMsg::Freeze {});
        match res.unwrap_err() {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but bob can
        let env = mock_env(&deps.api, &bob, &[]);
        handle(&mut deps, env, HandleMsg::Freeze {}).unwrap();
        let expected = ConfigResponse {
            admins: vec![alice.clone(), bob.clone()],
            mutable: false,
        };
        assert_eq!(query_config(&deps).unwrap(), expected);

        // and now alice cannot change it again
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![alice.clone()],
        };
        let env = mock_env(&deps.api, &alice, &[]);
        let res = handle(&mut deps, env, msg);
        match res.unwrap_err() {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn execute_messages_has_proper_permissions() {
        let mut deps = mock_dependencies(CANONICAL_LENGTH, &[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let carl = HumanAddr::from("carl");

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), carl.clone()],
            mutable: false,
        };
        let env = mock_env(&deps.api, &bob, &[]);
        init(&mut deps, env, init_msg).unwrap();

        let freeze: HandleMsg<Empty> = HandleMsg::Freeze {};
        let msgs = vec![
            BankMsg::Send {
                from_address: HumanAddr::from(MOCK_CONTRACT_ADDR),
                to_address: bob.clone(),
                amount: coins(10000, "DAI"),
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: HumanAddr::from("some contract"),
                msg: to_binary(&freeze).unwrap(),
                send: vec![],
            }
            .into(),
        ];

        // make some nice message
        let handle_msg = HandleMsg::Execute { msgs: msgs.clone() };

        // bob cannot execute them
        let env = mock_env(&deps.api, &bob, &[]);
        let res = handle(&mut deps, env, handle_msg.clone());
        match res.unwrap_err() {
            StdError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but carl can
        let env = mock_env(&deps.api, &carl, &[]);
        let res = handle(&mut deps, env, handle_msg.clone()).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(res.log, vec![log("action", "execute")]);
    }
}
