use schemars::JsonSchema;
use std::fmt;

use cosmwasm_std::{
    attr, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Empty, Env, HumanAddr,
    MessageInfo, Response, StdResult,
};
use cw1::CanExecuteResponse;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{AdminListResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{AdminList, ADMIN_LIST};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1-whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = AdminList {
        admins: map_canonical(deps.api, &msg.admins)?,
        mutable: msg.mutable,
    };
    ADMIN_LIST.save(deps.storage, &cfg)?;
    Ok(Response::default())
}

pub fn map_canonical(api: &dyn Api, admins: &[HumanAddr]) -> StdResult<Vec<CanonicalAddr>> {
    admins
        .iter()
        .map(|addr| api.canonical_address(addr))
        .collect()
}

fn map_human(api: &dyn Api, admins: &[CanonicalAddr]) -> StdResult<Vec<HumanAddr>> {
    admins.iter().map(|addr| api.human_address(addr)).collect()
}

pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: ExecuteMsg<Empty>,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Execute { msgs } => execute_execute(deps, env, info, msgs),
        ExecuteMsg::Freeze {} => execute_freeze(deps, env, info),
        ExecuteMsg::UpdateAdmins { admins } => execute_update_admins(deps, env, info, admins),
    }
}

pub fn execute_execute<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    if !can_execute(deps.as_ref(), &info.sender)? {
        Err(ContractError::Unauthorized {})
    } else {
        let mut res = Response::default();
        res.messages = msgs;
        res.attributes = vec![attr("action", "execute")];
        Ok(res)
    }
}

pub fn execute_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.can_modify(&deps.api.canonical_address(&info.sender)?) {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.mutable = false;
        ADMIN_LIST.save(deps.storage, &cfg)?;

        let mut res = Response::default();
        res.attributes = vec![attr("action", "freeze")];
        Ok(res)
    }
}

pub fn execute_update_admins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Vec<HumanAddr>,
) -> Result<Response, ContractError> {
    let mut cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.can_modify(&deps.api.canonical_address(&info.sender)?) {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.admins = map_canonical(deps.api, &admins)?;
        ADMIN_LIST.save(deps.storage, &cfg)?;

        let mut res = Response::default();
        res.attributes = vec![attr("action", "update_admins")];
        Ok(res)
    }
}

fn can_execute(deps: Deps, sender: &HumanAddr) -> StdResult<bool> {
    let cfg = ADMIN_LIST.load(deps.storage)?;
    let can = cfg.is_admin(&deps.api.canonical_address(sender)?);
    Ok(can)
}

pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::CanExecute { sender, msg } => to_binary(&query_can_execute(deps, sender, msg)?),
    }
}

pub fn query_admin_list(deps: Deps) -> StdResult<AdminListResponse> {
    let cfg = ADMIN_LIST.load(deps.storage)?;
    Ok(AdminListResponse {
        admins: map_human(deps.api, &cfg.admins)?,
        mutable: cfg.mutable,
    })
}

pub fn query_can_execute(
    deps: Deps,
    sender: HumanAddr,
    _msg: CosmosMsg,
) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, &sender)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, BankMsg, StakingMsg, WasmMsg};

    #[test]
    fn instantiate_and_modify_config() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let carl = HumanAddr::from("carl");

        let anyone = HumanAddr::from("anyone");

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            mutable: true,
        };
        let info = mock_info(&anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // anyone cannot modify the contract
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![anyone.clone()],
        };
        let info = mock_info(&anyone, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but alice can kick out carl
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.clone(), bob.clone()],
        };
        let info = mock_info(&alice, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // carl cannot freeze it
        let info = mock_info(&carl, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {});
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but bob can
        let info = mock_info(&bob, &[]);
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {}).unwrap();
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone()],
            mutable: false,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // and now alice cannot change it again
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.clone()],
        };
        let info = mock_info(&alice, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, msg);
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }
    }

    #[test]
    fn execute_messages_has_proper_permissions() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let carl = HumanAddr::from("carl");

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.clone(), carl.clone()],
            mutable: false,
        };
        let info = mock_info(&bob, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        let freeze: ExecuteMsg<Empty> = ExecuteMsg::Freeze {};
        let msgs = vec![
            BankMsg::Send {
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
        let execute_msg = ExecuteMsg::Execute { msgs: msgs.clone() };

        // bob cannot execute them
        let info = mock_info(&bob, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone());
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but carl can
        let info = mock_info(&carl, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(res.attributes, vec![attr("action", "execute")]);
    }

    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");

        let anyone = HumanAddr::from("anyone");

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.clone(), bob.clone()],
            mutable: false,
        };
        let info = mock_info(&anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.clone(),
            amount: coins(12345, "ushell"),
        });
        let staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.clone(),
            amount: coin(70000, "ureef"),
        });

        // owner can send
        let res = query_can_execute(deps.as_ref(), alice.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_execute, true);

        // owner can stake
        let res = query_can_execute(deps.as_ref(), bob.clone(), staking_msg.clone()).unwrap();
        assert_eq!(res.can_execute, true);

        // anyone cannot send
        let res = query_can_execute(deps.as_ref(), anyone.clone(), send_msg.clone()).unwrap();
        assert_eq!(res.can_execute, false);

        // anyone cannot stake
        let res = query_can_execute(deps.as_ref(), anyone.clone(), staking_msg.clone()).unwrap();
        assert_eq!(res.can_execute, false);
    }
}
