#![allow(dead_code)]
use cosmwasm_std::{CosmosMsg, Response};

pub struct Ctx;
pub struct Error;

#[cw_derive::interface(module=msg, exec=Cw1Exec, query=Cw1Query)]
pub trait Cw1<Msg>
where
    Msg: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
{
    #[msg(exec)]
    fn execute(&self, ctx: Ctx, msgs: Vec<CosmosMsg<Msg>>) -> Result<Response, Error>;
}

fn main() {}
