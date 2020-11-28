use schemars::JsonSchema;
use std::fmt;

use cosmwasm_std::{
    attr, to_binary, Api, Binary, CanonicalAddr, CosmosMsg, Deps, DepsMut, Empty, Env,
    HandleResponse, HumanAddr, InitResponse, MessageInfo, StdResult,
};
use cw1::CanExecuteResponse;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{AdminListResponse, HandleMsg, InitMsg, QueryMsg};
use crate::state::{admin_list, admin_list_read, AdminList};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1-whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn init(deps: DepsMut, _env: Env, _info: MessageInfo, msg: InitMsg) -> StdResult<InitResponse> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let cfg = AdminList {
        admins: map_canonical(deps.api, &msg.admins)?,
        mutable: msg.mutable,
    };
    admin_list(deps.storage).save(&cfg)?;
    Ok(InitResponse::default())
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

pub fn handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: HandleMsg<Empty>,
) -> Result<HandleResponse<Empty>, ContractError> {
    match msg {
        HandleMsg::Execute { msgs } => handle_execute(deps, env, info, msgs),
        HandleMsg::Freeze {} => handle_freeze(deps, env, info),
        HandleMsg::UpdateAdmins { admins } => handle_update_admins(deps, env, info, admins),
    }
}

pub fn handle_execute<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<HandleResponse<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    if !can_execute(deps.as_ref(), &info.sender)? {
        Err(ContractError::Unauthorized {})
    } else {
        let mut res = HandleResponse::default();
        res.messages = msgs;
        res.attributes = vec![attr("action", "execute")];
        Ok(res)
    }
}

pub fn handle_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<HandleResponse, ContractError> {
    let mut cfg = admin_list_read(deps.storage).load()?;
    if !cfg.can_modify(&deps.api.canonical_address(&info.sender)?) {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.mutable = false;
        admin_list(deps.storage).save(&cfg)?;

        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "freeze")];
        Ok(res)
    }
}

pub fn handle_update_admins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Vec<HumanAddr>,
) -> Result<HandleResponse, ContractError> {
    let mut cfg = admin_list_read(deps.storage).load()?;
    if !cfg.can_modify(&deps.api.canonical_address(&info.sender)?) {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.admins = map_canonical(deps.api, &admins)?;
        admin_list(deps.storage).save(&cfg)?;

        let mut res = HandleResponse::default();
        res.attributes = vec![attr("action", "update_admins")];
        Ok(res)
    }
}

fn can_execute(deps: Deps, sender: &HumanAddr) -> StdResult<bool> {
    let cfg = admin_list_read(deps.storage).load()?;
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
    let cfg = admin_list_read(deps.storage).load()?;
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
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{coin, coins, BankMsg, StakingMsg, WasmMsg};

    #[test]
    fn init_and_modify_config() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");
        let carl = HumanAddr::from("carl");

        let anyone = HumanAddr::from("anyone");

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            mutable: true,
        };
        let info = mock_info(&anyone, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone(), carl.clone()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // anyone cannot modify the contract
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![anyone.clone()],
        };
        let info = mock_info(&anyone, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, msg);
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but alice can kick out carl
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![alice.clone(), bob.clone()],
        };
        let info = mock_info(&alice, &[]);
        handle(deps.as_mut(), mock_env(), info, msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone()],
            mutable: true,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // carl cannot freeze it
        let info = mock_info(&carl, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, HandleMsg::Freeze {});
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but bob can
        let info = mock_info(&bob, &[]);
        handle(deps.as_mut(), mock_env(), info, HandleMsg::Freeze {}).unwrap();
        let expected = AdminListResponse {
            admins: vec![alice.clone(), bob.clone()],
            mutable: false,
        };
        assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // and now alice cannot change it again
        let msg = HandleMsg::UpdateAdmins {
            admins: vec![alice.clone()],
        };
        let info = mock_info(&alice, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, msg);
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

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), carl.clone()],
            mutable: false,
        };
        let info = mock_info(&bob, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

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
        let info = mock_info(&bob, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, handle_msg.clone());
        match res.unwrap_err() {
            ContractError::Unauthorized { .. } => {}
            e => panic!("unexpected error: {}", e),
        }

        // but carl can
        let info = mock_info(&carl, &[]);
        let res = handle(deps.as_mut(), mock_env(), info, handle_msg.clone()).unwrap();
        assert_eq!(res.messages, msgs);
        assert_eq!(res.attributes, vec![attr("action", "execute")]);
    }

    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies(&[]);

        let alice = HumanAddr::from("alice");
        let bob = HumanAddr::from("bob");

        let anyone = HumanAddr::from("anyone");

        // init the contract
        let init_msg = InitMsg {
            admins: vec![alice.clone(), bob.clone()],
            mutable: false,
        };
        let info = mock_info(&anyone, &[]);
        init(deps.as_mut(), mock_env(), info, init_msg).unwrap();

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            from_address: MOCK_CONTRACT_ADDR.into(),
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
