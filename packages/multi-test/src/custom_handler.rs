use cosmwasm_std::{Addr, Api, Binary, BlockInfo, Empty, Storage};
use mockall::automock;

use anyhow::Result as AnyResult;

use crate::AppResponse;

/// Custom message handler trait. Implementator of this trait is mocking environment behaviour on
/// given custom message.
pub trait CustomHandler<ExecC = Empty, QueryC = Empty> {
    fn execute(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: ExecC,
    ) -> AnyResult<AppResponse>;

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        block: &BlockInfo,
        msg: QueryC,
    ) -> AnyResult<Binary>;
}

/// Simplified version of `CustomHandler` having only arguments which are not app internals - they
/// are just discarded. Usefull for simpler mocking.
#[automock(type QueryResult = Binary;)]
pub trait SimpleCustomHandler<ExecC = Empty, QueryC = Empty> {
    type QueryResult;

    fn execute(&self, block: &BlockInfo, sender: Addr, msg: ExecC) -> AnyResult<AppResponse>;
    fn query(&self, block: &BlockInfo, msg: QueryC) -> AnyResult<Binary>;
}

impl<ExecC, QueryC, T: SimpleCustomHandler<ExecC, QueryC>> CustomHandler<ExecC, QueryC> for T {
    fn execute(
        &self,
        _: &dyn Api,
        _: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: ExecC,
    ) -> AnyResult<AppResponse> {
        self.execute(block, sender, msg)
    }

    fn query(
        &self,
        _: &dyn Api,
        _: &dyn Storage,
        block: &BlockInfo,
        msg: QueryC,
    ) -> AnyResult<Binary> {
        self.query(block, msg)
    }
}
