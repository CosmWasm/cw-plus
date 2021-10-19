use cosmwasm_std::{Addr, Api, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

use crate::error::ContractError;
use crate::interfaces::Cw1Whitelist;
use crate::msg::AdminListResponse;
use crate::state::{AdminList, Cw1WhitelistContract};

use cw1::CanExecuteResponse;
use cw2::set_contract_version;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1-whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

fn validate_admins(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(&addr)).collect()
}

impl Cw1WhitelistContract {
    pub fn instantiate(
        &self,
        deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        admins: Vec<String>,
        mutable: bool,
    ) -> Result<Response, ContractError> {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
        let cfg = AdminList {
            admins: validate_admins(deps.api, &admins)?,
            mutable,
        };

        self.admin_list.save(deps.storage, &cfg)?;
        Ok(Response::new())
    }

    fn is_admin(&self, deps: Deps, addr: &str) -> Result<bool, ContractError> {
        let cfg = self.admin_list().load(deps.storage)?;
        Ok(cfg.is_admin(addr))
    }
}

impl<T> Cw1Whitelist<T> for Cw1WhitelistContract {
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
