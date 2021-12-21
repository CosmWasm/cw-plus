#![allow(dead_code)]
use cosmwasm_std::{CosmosMsg, Response};

struct Ctx;
struct Error;

#[cw_derive::interface(module=msg, exec=Cw1Exec, query=Cw1Query)]
trait Cw1<Msg>
where
    Msg: std::fmt::Debug + PartialEq + Clone + schemars::JsonSchema,
{
    #[msg(exec)]
    fn execute(&self, ctx: Ctx, msgs: Vec<CosmosMsg<Msg>>) -> Result<Response, Error>;
}

fn main() {}
