use cosmwasm_std::{Addr, Decimal, Response};

pub struct Ctx;
pub struct Error;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
pub struct QueryResult;

#[cw_derive::interface(exec=IfaceExec, query=IfaceQuery)]
pub trait Interface {
    #[msg(exec)]
    fn no_args_execution(&self, ctx: Ctx) -> Result<Response, Error>;

    #[msg(exec)]
    fn argumented_execution(&self, ctx: Ctx, addr: Addr, coef: Decimal, desc: String);

    #[msg(query)]
    fn no_args_query(&self, ctx: Ctx) -> Result<QueryResult, Error>;

    #[msg(query)]
    fn argumented_query(&self, ctx: Ctx, user: Addr) -> Result<QueryResult, Error>;
}

#[test]
fn messages_constructible() {
    let _no_args_exec = IfaceExec::NoArgsExecution {};
    let _argumented_exec = IfaceExec::ArgumentedExecution {
        addr: Addr::unchecked("owner"),
        coef: Decimal::percent(10),
        desc: "Some description".to_owned(),
    };
    let _no_args_query = IfaceQuery::NoArgsQuery {};
    let _argumented_query = IfaceQuery::ArgumentedQuery {
        user: Addr::unchecked("owner"),
    };
}
