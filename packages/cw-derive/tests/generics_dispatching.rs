use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{
    from_binary, Addr, Deps, DepsMut, Env, MessageInfo, Response, StdError, Uint128,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;

#[cw_derive::interface]
pub trait Interface<T>
where
    T: Serialize,
{
    type Error: From<StdError>;

    #[msg(exec)]
    fn store(
        &self,
        ctx: (DepsMut, Env, MessageInfo),
        addr: Addr,
        data: T,
    ) -> Result<Response, Self::Error>;

    #[msg(query)]
    fn load(&self, ctx: (Deps, Env), addr: Addr) -> Result<QueryResponse<T>, Self::Error>;
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Std(StdError),
    MissingData(Addr),
}

impl From<StdError> for Error {
    fn from(src: StdError) -> Self {
        Self::Std(src)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct QueryResponse<T> {
    data: T,
}

#[derive(Default)]
pub struct Contract<T> {
    data: RefCell<HashMap<Addr, T>>,
}

impl<T> Interface<T> for Contract<T>
where
    T: Serialize + Clone,
{
    type Error = Error;

    fn store(
        &self,
        _: (DepsMut, Env, MessageInfo),
        addr: Addr,
        data: T,
    ) -> Result<Response, Error> {
        self.data.borrow_mut().insert(addr, data);
        Ok(Response::new())
    }

    fn load(&self, _: (Deps, Env), addr: Addr) -> Result<QueryResponse<T>, Error> {
        let data = self
            .data
            .borrow()
            .get(&addr)
            .ok_or(Error::MissingData(addr))?
            .clone();
        Ok(QueryResponse { data })
    }
}

#[test]
fn dispatch_on_string() {
    let contract: Contract<String> = Contract::default();

    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("owner", &[]);

    ExecMsg::Store {
        addr: Addr::unchecked("addr1"),
        data: "str1".to_owned(),
    }
    .dispatch(&contract, (deps.as_mut(), env.clone(), info.clone()))
    .unwrap();

    ExecMsg::Store {
        addr: Addr::unchecked("addr2"),
        data: "str2".to_owned(),
    }
    .dispatch(&contract, (deps.as_mut(), env.clone(), info))
    .unwrap();

    let resp = QueryMsg::Load {
        addr: Addr::unchecked("addr2"),
    }
    .dispatch(&contract, (deps.as_ref(), env.clone()))
    .unwrap();
    let resp: QueryResponse<String> = from_binary(&resp).unwrap();
    assert_eq!(
        resp,
        QueryResponse {
            data: "str2".to_owned()
        }
    );

    let err = QueryMsg::Load {
        addr: Addr::unchecked("addr3"),
    }
    .dispatch(&contract, (deps.as_ref(), env))
    .unwrap_err();
    assert_eq!(err, Error::MissingData(Addr::unchecked("addr3")),);
}

#[test]
fn dispatch_on_uint128() {
    let contract: Contract<Uint128> = Contract::default();

    let mut deps = mock_dependencies();
    let env = mock_env();
    let info = mock_info("owner", &[]);

    ExecMsg::Store {
        addr: Addr::unchecked("addr1"),
        data: 100u128.into(),
    }
    .dispatch(&contract, (deps.as_mut(), env.clone(), info.clone()))
    .unwrap();

    ExecMsg::Store {
        addr: Addr::unchecked("addr2"),
        data: 200u128.into(),
    }
    .dispatch(&contract, (deps.as_mut(), env.clone(), info))
    .unwrap();

    let resp = QueryMsg::Load {
        addr: Addr::unchecked("addr2"),
    }
    .dispatch(&contract, (deps.as_ref(), env.clone()))
    .unwrap();
    let resp: QueryResponse<Uint128> = from_binary(&resp).unwrap();
    assert_eq!(
        resp,
        QueryResponse {
            data: 200u128.into()
        }
    );

    let err = QueryMsg::Load {
        addr: Addr::unchecked("addr3"),
    }
    .dispatch(&contract, (deps.as_ref(), env))
    .unwrap_err();
    assert_eq!(err, Error::MissingData(Addr::unchecked("addr3")),);
}
