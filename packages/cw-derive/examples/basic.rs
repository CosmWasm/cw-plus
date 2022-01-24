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

#[cw_derive::interface(exec=Execute, query=Query)]
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

pub struct GroupContract<'a> {
    admin: Item<'a, Addr>,
    members: Map<'a, Addr, u64>,
}

impl<'a> Default for GroupContract<'a> {
    fn default() -> Self {
        Self::new()
    }
}

#[cw_derive::contract]
impl<'a> GroupContract<'a> {
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
