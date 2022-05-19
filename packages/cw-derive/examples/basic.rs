#![allow(dead_code)]
use anyhow::Error;
use cosmwasm_std::{Addr, Deps, DepsMut, Env, MessageInfo, Response, StdError};
use cw_storage_plus::{Item, Map};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
pub struct Member {
    addr: String,
    weight: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
pub struct MemberResp {
    weight: u64,
}

#[cw_derive::interface(module=group)]
pub trait Group {
    type Error: From<StdError>;

    #[msg(exec)]
    fn update_admin(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        admin: Option<String>,
    ) -> Result<Response, Self::Error>;

    #[msg(exec)]
    fn update_members(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        remove: Vec<String>,
        add: Vec<Member>,
    ) -> Result<Response, Self::Error>;

    #[msg(query)]
    fn member(&self, ctx: (Deps, Env), addr: String) -> Result<MemberResp, Self::Error>;
}

pub struct GroupContract {
    admin: Item<'static, Addr>,
    members: Map<'static, Addr, u64>,
}

impl Default for GroupContract {
    fn default() -> Self {
        Self::new()
    }
}

impl Group for GroupContract {
    type Error = Error;

    fn update_admin(
        &self,
        _ctx: (DepsMut, Env, MessageInfo),
        _admin: Option<String>,
    ) -> Result<Response, Self::Error> {
        todo!()
    }

    fn update_members(
        &self,
        _ctx: (DepsMut, Env, MessageInfo),
        _remove: Vec<String>,
        _add: Vec<Member>,
    ) -> Result<Response, Self::Error> {
        todo!()
    }

    fn member(&self, _ctx: (Deps, Env), _addr: String) -> Result<MemberResp, Self::Error> {
        todo!()
    }
}

#[cw_derive::contract(module=contract, error=Error)]
#[messages(group as Group)]
impl GroupContract {
    pub fn new() -> Self {
        Self {
            admin: Item::new("admin"),
            members: Map::new("members"),
        }
    }

    #[msg(instantiate)]
    pub fn instantiate(
        &self,
        (deps, _env, _msg): (DepsMut, Env, MessageInfo),
        admin: Option<String>,
    ) -> Result<Response, Error> {
        if let Some(admin) = admin {
            let admin = deps.api.addr_validate(&admin)?;
            self.admin.save(deps.storage, &admin)?;
        }

        Ok(Response::new())
    }
}

fn main() {}
