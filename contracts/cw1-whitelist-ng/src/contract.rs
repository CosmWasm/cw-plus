use cosmwasm_std::{Addr, Api, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::interfaces::*;
use crate::msg::AdminListResponse;
use crate::state::{AdminList, Cw1WhitelistContract};

use cw1::CanExecuteResponse;
use cw2::set_contract_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1-whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn validate_admins(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(&addr)).collect()
}

impl<T> Cw1WhitelistContract<T> {
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        admins: Vec<String>,
        mutable: bool,
    ) -> Result<Response<T>, ContractError> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        let cfg = AdminList {
            admins: validate_admins(deps.api, &admins)?,
            mutable,
        };

        self.admin_list.save(deps.storage, &cfg)?;
        Ok(Response::new())
    }

    pub fn is_admin(&self, deps: Deps, addr: &str) -> Result<bool, ContractError> {
        let cfg = self.admin_list.load(deps.storage)?;
        Ok(cfg.is_admin(addr))
    }
}

impl<T> Cw1<T> for Cw1WhitelistContract<T> {
    type Error = ContractError;

    fn execute(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        msgs: Vec<CosmosMsg<T>>,
    ) -> Result<Response<T>, Self::Error> {
        if !self.is_admin(deps.as_ref(), info.sender.as_ref())? {
            Err(ContractError::Unauthorized {})
        } else {
            let res = Response::new()
                .add_messages(msgs)
                .add_attribute("action", "execute");
            Ok(res)
        }
    }

    fn can_execute(
        &self,
        deps: Deps,
        _env: Env,
        sender: String,
        _msg: CosmosMsg<T>,
    ) -> Result<CanExecuteResponse, Self::Error> {
        Ok(CanExecuteResponse {
            can_execute: self.is_admin(deps, &sender)?,
        })
    }
}

impl<T> Whitelist<T> for Cw1WhitelistContract<T> {
    type Error = ContractError;

    fn freeze(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
    ) -> Result<Response<T>, Self::Error> {
        self.admin_list
            .update(deps.storage, |mut cfg| -> Result<_, ContractError> {
                if !cfg.can_modify(info.sender.as_str()) {
                    Err(ContractError::Unauthorized {})
                } else {
                    cfg.mutable = false;
                    Ok(cfg)
                }
            })?;

        Ok(Response::new().add_attribute("action", "freeze"))
    }

    fn update_admins(
        &self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        admins: Vec<String>,
    ) -> Result<Response<T>, Self::Error> {
        let api = deps.api;
        self.admin_list
            .update(deps.storage, |mut cfg| -> Result<_, ContractError> {
                if !cfg.can_modify(info.sender.as_str()) {
                    Err(ContractError::Unauthorized {})
                } else {
                    cfg.admins = validate_admins(api, &admins)?;
                    Ok(cfg)
                }
            })?;

        Ok(Response::new().add_attribute("action", "update_admins"))
    }

    fn admin_list(&self, deps: Deps, _env: Env) -> Result<AdminListResponse, Self::Error> {
        let cfg = self.admin_list.load(deps.storage)?;
        Ok(AdminListResponse {
            admins: cfg.admins.into_iter().map(|a| a.into()).collect(),
            mutable: cfg.mutable,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, to_binary, BankMsg, StakingMsg, SubMsg, WasmMsg};

    #[test]
    fn instantiate_and_modify_config() {
        let contract = Cw1WhitelistContract::native();

        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        let anyone = "anyone";

        // instantiate the contract
        let admins = vec![alice.to_owned(), bob.to_owned(), carl.to_owned()];
        let info = mock_info(&anyone, &[]);
        contract
            .instantiate(deps.as_mut(), mock_env(), info, admins, true)
            .unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string(), carl.to_string()],
            mutable: true,
        };
        assert_eq!(
            contract.admin_list(deps.as_ref(), mock_env()).unwrap(),
            expected
        );

        // anyone cannot modify the contract
        let info = mock_info(&anyone, &[]);
        let err = contract
            .update_admins(deps.as_mut(), mock_env(), info, vec![anyone.to_owned()])
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but alice can kick out carl
        let admins = vec![alice.to_owned(), bob.to_owned()];
        let info = mock_info(&alice, &[]);
        contract
            .update_admins(deps.as_mut(), mock_env(), info, admins)
            .unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: true,
        };
        assert_eq!(
            contract.admin_list(deps.as_ref(), mock_env()).unwrap(),
            expected
        );

        // carl cannot freeze it
        let info = mock_info(&carl, &[]);
        let err = contract
            .freeze(deps.as_mut(), mock_env(), info)
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but bob can
        let info = mock_info(&bob, &[]);
        contract.freeze(deps.as_mut(), mock_env(), info).unwrap();
        let expected = AdminListResponse {
            admins: vec![alice.to_owned(), bob.to_owned()],
            mutable: false,
        };
        assert_eq!(
            contract.admin_list(deps.as_ref(), mock_env()).unwrap(),
            expected
        );

        // and now alice cannot change it again
        let info = mock_info(&alice, &[]);
        let err = contract
            .update_admins(deps.as_mut(), mock_env(), info, vec![alice.to_owned()])
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn execute_messages_has_proper_permissions() {
        let contract = Cw1WhitelistContract::native();
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        // instantiate the contract
        let admins = vec![alice.to_owned(), carl.to_owned()];
        let info = mock_info(&bob, &[]);
        contract
            .instantiate(deps.as_mut(), mock_env(), info, admins, false)
            .unwrap();

        let freeze = WhitelistExecMsg::Freeze {};
        let msgs = vec![
            BankMsg::Send {
                to_address: bob.to_string(),
                amount: coins(10000, "DAI"),
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "some contract".into(),
                msg: to_binary(&freeze).unwrap(),
                funds: vec![],
            }
            .into(),
        ];

        // bob cannot execute them
        let info = mock_info(&bob, &[]);
        let err = contract
            .execute(deps.as_mut(), mock_env(), info, msgs.clone())
            .unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but carl can
        let info = mock_info(&carl, &[]);
        let res = contract
            .execute(deps.as_mut(), mock_env(), info, msgs.clone())
            .unwrap();
        assert_eq!(
            res.messages,
            msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
        );
        assert_eq!(res.attributes, [("action", "execute")]);
    }

    #[test]
    fn execute_custom_messages_works() {
        let contract = Cw1WhitelistContract::<String>::new();
        let mut deps = mock_dependencies();
        let alice = "alice";

        let admins = vec![alice.to_owned()];
        let info = mock_info(&alice, &[]);
        contract
            .instantiate(deps.as_mut(), mock_env(), info, admins, false)
            .unwrap();

        let msgs = vec![CosmosMsg::Custom("msg".to_owned())];

        let res = contract
            .execute(
                deps.as_mut(),
                mock_env(),
                mock_info(&alice, &[]),
                msgs.clone(),
            )
            .unwrap();

        assert_eq!(
            res.messages,
            msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
        );
    }

    #[test]
    fn can_execute_query_works() {
        let contract = Cw1WhitelistContract::native();
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";

        let anyone = "anyone";

        // instantiate the contract
        let admins = vec![alice.to_owned(), bob.to_owned()];
        let info = mock_info(&anyone, &[]);
        contract
            .instantiate(deps.as_mut(), mock_env(), info, admins, false)
            .unwrap();

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.to_string(),
            amount: coins(12345, "ushell"),
        });
        let staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.to_string(),
            amount: coin(70000, "ureef"),
        });

        // owner can send
        let res = contract
            .can_execute(
                deps.as_ref(),
                mock_env(),
                alice.to_owned(),
                send_msg.clone(),
            )
            .unwrap();
        assert!(res.can_execute);

        // owner can stake
        let res = contract
            .can_execute(
                deps.as_ref(),
                mock_env(),
                bob.to_owned(),
                staking_msg.clone(),
            )
            .unwrap();
        assert!(res.can_execute);

        // anyone cannot send
        let res = contract
            .can_execute(deps.as_ref(), mock_env(), anyone.to_owned(), send_msg)
            .unwrap();
        assert!(!res.can_execute);

        // anyone cannot stake
        let res = contract
            .can_execute(deps.as_ref(), mock_env(), anyone.to_owned(), staking_msg)
            .unwrap();
        assert!(!res.can_execute);
    }
}
