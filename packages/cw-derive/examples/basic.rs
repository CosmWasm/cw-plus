#![allow(dead_code)]
use cosmwasm_std::{DepsMut, Env, MessageInfo, Response};

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
pub struct Member;

#[cw_derive::interface(module=msg, exec=Execute, query=Query)]
pub trait Cw4 {
    type Error;

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

    #[msg(exec)]
    fn add_hook(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        addr: String,
    ) -> Result<Response, Self::Error>;

    #[msg(exec)]
    fn remove_hook(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        addr: String,
    ) -> Result<Response, Self::Error>;
}

pub struct Cw4Contract {
    admin: 
}

impl Cw4Contract {
        fn instantiate(&self, (deps, env, msg): (DepsMut, Env, MessageInfo),
}

fn main() {}
