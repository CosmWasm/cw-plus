use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, CustomQuery, Querier, QuerierWrapper, StdResult, Uint128, WasmMsg,
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

    /// Get token balance for the given address
    pub fn balance<Q, T, CQ>(&self, querier: &Q, address: T) -> StdResult<Uint128>
    where
        Q: Querier,
        T: Into<String>,
        CQ: CustomQuery,
    {
        let msg = Cw20QueryMsg::Balance {
            address: address.into(),
        };
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        let res: BalanceResponse = QuerierWrapper::<CQ>::new(querier).query(&query)?;
        Ok(res.balance)
    }

    /// Get metadata from the contract. This is a good check that the address
    /// is a valid Cw20 contract.
    pub fn meta<Q, CQ>(&self, querier: &Q) -> StdResult<TokenInfoResponse>
    where
        Q: Querier,
        CQ: CustomQuery,
    {
        let msg = Cw20QueryMsg::TokenInfo {};
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        QuerierWrapper::<CQ>::new(querier).query(&query)
    }

    /// Get allowance of spender to use owner's account
    pub fn allowance<Q, T, U, CQ>(
        &self,
        querier: &Q,
        owner: T,
        spender: U,
    ) -> StdResult<AllowanceResponse>
    where
        Q: Querier,
        T: Into<String>,
        U: Into<String>,
        CQ: CustomQuery,
    {
        let msg = Cw20QueryMsg::Allowance {
            owner: owner.into(),
            spender: spender.into(),
        };
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        QuerierWrapper::<CQ>::new(querier).query(&query)
    }

    /// Find info on who can mint, and how much
    pub fn minter<Q, CQ>(&self, querier: &Q) -> StdResult<Option<MinterResponse>>
    where
        Q: Querier,
        CQ: CustomQuery,
    {
        let msg = Cw20QueryMsg::Minter {};
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&msg)?,
        }
        .into();
        QuerierWrapper::<CQ>::new(querier).query(&query)
    }

    /// returns true if the contract supports the allowance extension
    pub fn has_allowance<Q: Querier, CQ: CustomQuery>(&self, querier: &Q) -> bool {
        self.allowance::<_, _, _, CQ>(querier, self.addr(), self.addr())
            .is_ok()
    }

    /// returns true if the contract supports the mintable extension
    pub fn is_mintable<Q: Querier, CQ: CustomQuery>(&self, querier: &Q) -> bool {
        self.minter::<_, CQ>(querier).is_ok()
    }
}
