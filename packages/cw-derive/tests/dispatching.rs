use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    from_binary, Addr, Decimal, Deps, DepsMut, Env, MessageInfo, Response, StdError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

#[cw_derive::interface]
pub trait Interface {
    type Error: From<StdError>;

    #[msg(exec)]
    fn no_args_execution(&self, ctx: (DepsMut, Env, MessageInfo)) -> Result<Response, Self::Error>;

    #[msg(exec)]
    fn argumented_execution(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        addr: Addr,
        coef: Decimal,
        desc: String,
    ) -> Result<Response, Self::Error>;

    #[msg(query)]
    fn no_args_query(&self, ctx: (Deps, Env)) -> Result<EmptyQueryResponse, Self::Error>;

    #[msg(query)]
    fn argumented_query(&self, ctx: (Deps, Env), user: Addr) -> Result<QueryResponse, Self::Error>;
}

#[derive(Default)]
struct Contract {
    execs: RefCell<u64>,
    queries: RefCell<u64>,

    data: RefCell<HashMap<Addr, QueryResponse>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct EmptyQueryResponse {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryResponse {
    coef: Decimal,
    desc: String,
}

impl Interface for Contract {
    type Error = StdError;

    fn no_args_execution(&self, _: (DepsMut, Env, MessageInfo)) -> Result<Response, StdError> {
        *self.execs.borrow_mut() += 1;
        Ok(Response::new())
    }

    fn argumented_execution(
        &self,
        _: (DepsMut, Env, MessageInfo),
        addr: Addr,
        coef: Decimal,
        desc: String,
    ) -> Result<Response, Self::Error> {
        *self.execs.borrow_mut() += 1;

        self.data
            .borrow_mut()
            .insert(addr, QueryResponse { coef, desc });
        Ok(Response::new())
    }

    fn no_args_query(&self, _: (Deps, Env)) -> Result<EmptyQueryResponse, StdError> {
        *self.queries.borrow_mut() += 1;
        Ok(dbg!(EmptyQueryResponse {}))
    }

    fn argumented_query(&self, _: (Deps, Env), user: Addr) -> Result<QueryResponse, Self::Error> {
        *self.queries.borrow_mut() += 1;
        Ok(self.data.borrow().get(&user).unwrap().clone())
    }
}

#[test]
fn dispatch() {
    let contract = Contract::default();

    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("owner", &[]);

    let resp = ExecMsg::NoArgsExecution {}
        .dispatch(&contract, (deps.as_mut(), env.clone(), info.clone()))
        .unwrap();
    assert_eq!(resp, Response::new());

    let resp = ExecMsg::ArgumentedExecution {
        addr: Addr::unchecked("addr1"),
        coef: Decimal::percent(30),
        desc: "True".to_owned(),
    }
    .dispatch(&contract, (deps.as_mut(), env.clone(), info.clone()))
    .unwrap();
    assert_eq!(resp, Response::new());

    let resp = ExecMsg::ArgumentedExecution {
        addr: Addr::unchecked("addr2"),
        coef: Decimal::percent(70),
        desc: "False".to_owned(),
    }
    .dispatch(&contract, (deps.as_mut(), env.clone(), info))
    .unwrap();
    assert_eq!(resp, Response::new());

    let resp = QueryMsg::NoArgsQuery {}
        .dispatch(&contract, (deps.as_ref(), env.clone()))
        .unwrap();
    let _resp: EmptyQueryResponse = from_binary(&resp).unwrap();

    let resp = QueryMsg::ArgumentedQuery {
        user: Addr::unchecked("addr2"),
    }
    .dispatch(&contract, (deps.as_ref(), env))
    .unwrap();
    let resp: QueryResponse = from_binary(&resp).unwrap();
    assert_eq!(
        resp,
        QueryResponse {
            coef: Decimal::percent(70),
            desc: "False".to_owned()
        }
    );

    assert_eq!(*contract.execs.borrow(), 3);
    assert_eq!(*contract.queries.borrow(), 2);
}
