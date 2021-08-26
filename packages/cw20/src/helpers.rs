use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Empty, QuerierWrapper, QueryRequest, StdResult, Uint128, WasmMsg,
    WasmQuery,
};

use crate::{
    AllowanceResponse, BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, MinterResponse,
    TokenInfoResponse,
};

/// Cw20Contract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
///
/// If you wish to persist this, convert to Cw20CanonicalContract via .canonical()
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw20Contract(pub Addr);

impl Cw20Contract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call<T: Into<Cw20ExecuteMsg>>(&self, msg: T) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg.into())?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    fn encode_smart_query(&self, msg: Cw20QueryMsg) -> StdResult<QueryRequest<Empty>> {
        Ok(WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into())
    }

    /// Get token balance for the given address
    pub fn balance<T: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        address: T,
    ) -> StdResult<Uint128> {
        let query = self.encode_smart_query(Cw20QueryMsg::Balance {
            address: address.into(),
        })?;
        let res: BalanceResponse = querier.query(&query)?;
        Ok(res.balance)
    }

    /// Get metadata from the contract. This is a good check that the address
    /// is a valid Cw20 contract.
    pub fn meta(&self, querier: &QuerierWrapper) -> StdResult<TokenInfoResponse> {
        let query = self.encode_smart_query(Cw20QueryMsg::TokenInfo {})?;
        querier.query(&query)
    }

    /// Get allowance of spender to use owner's account
    pub fn allowance<T: Into<String>, U: Into<String>>(
        &self,
        querier: &QuerierWrapper,
        owner: T,
        spender: U,
    ) -> StdResult<AllowanceResponse> {
        let query = self.encode_smart_query(Cw20QueryMsg::Allowance {
            owner: owner.into(),
            spender: spender.into(),
        })?;
        querier.query(&query)
    }

    /// Find info on who can mint, and how much
    pub fn minter(&self, querier: &QuerierWrapper) -> StdResult<Option<MinterResponse>> {
        let query = self.encode_smart_query(Cw20QueryMsg::Minter {})?;
        querier.query(&query)
    }

    /// returns true if the contract supports the allowance extension
    pub fn has_allowance(&self, querier: &QuerierWrapper) -> bool {
        self.allowance(querier, self.addr(), self.addr()).is_ok()
    }

    /// returns true if the contract supports the mintable extension
    pub fn is_mintable(&self, querier: &QuerierWrapper) -> bool {
        self.minter(querier).is_ok()
    }
}
