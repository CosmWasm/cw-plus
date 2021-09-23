use schemars::JsonSchema;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Querier, QuerierWrapper, StdResult, WasmMsg, WasmQuery,
};

use crate::{
    AllNftInfoResponse, Approval, ApprovedForAllResponse, ContractInfoResponse, Cw721ExecuteMsg,
    Cw721QueryMsg, NftInfoResponse, NumTokensResponse, OwnerOfResponse, TokensResponse,
};

/// Cw721Contract is a wrapper around Addr that provides a lot of helpers
/// for working with this.
///
/// If you wish to persist this, convert to Cw721CanonicalContract via .canonical()
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Cw721Contract(pub Addr);

impl Cw721Contract {
    pub fn addr(&self) -> Addr {
        self.0.clone()
    }

    pub fn call(&self, msg: Cw721ExecuteMsg) -> StdResult<CosmosMsg> {
        let msg = to_binary(&msg)?;
        Ok(WasmMsg::Execute {
            contract_addr: self.addr().into(),
            msg,
            funds: vec![],
        }
        .into())
    }

    pub fn query<Q: Querier, T: DeserializeOwned>(
        &self,
        querier: &Q,
        req: Cw721QueryMsg,
    ) -> StdResult<T> {
        let query = WasmQuery::Smart {
            contract_addr: self.addr().into(),
            msg: to_binary(&req)?,
        }
        .into();
        QuerierWrapper::new(querier).query(&query)
    }

    /*** queries ***/

    pub fn owner_of<Q: Querier, T: Into<String>>(
        &self,
        querier: &Q,
        token_id: T,
        include_expired: bool,
    ) -> StdResult<OwnerOfResponse> {
        let req = Cw721QueryMsg::OwnerOf {
            token_id: token_id.into(),
            include_expired: Some(include_expired),
        };
        self.query(querier, req)
    }

    pub fn approved_for_all<Q: Querier, T: Into<String>>(
        &self,
        querier: &Q,
        owner: T,
        include_expired: bool,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<Vec<Approval>> {
        let req = Cw721QueryMsg::ApprovedForAll {
            owner: owner.into(),
            include_expired: Some(include_expired),
            start_after,
            limit,
        };
        let res: ApprovedForAllResponse = self.query(querier, req)?;
        Ok(res.operators)
    }

    pub fn num_tokens<Q: Querier>(&self, querier: &Q) -> StdResult<u64> {
        let req = Cw721QueryMsg::NumTokens {};
        let res: NumTokensResponse = self.query(querier, req)?;
        Ok(res.count)
    }

    /// With metadata extension
    pub fn contract_info<Q: Querier>(&self, querier: &Q) -> StdResult<ContractInfoResponse> {
        let req = Cw721QueryMsg::ContractInfo {};
        self.query(querier, req)
    }

    /// With metadata extension
    pub fn nft_info<Q: Querier, T: Into<String>, U: DeserializeOwned>(
        &self,
        querier: &Q,
        token_id: T,
    ) -> StdResult<NftInfoResponse<U>> {
        let req = Cw721QueryMsg::NftInfo {
            token_id: token_id.into(),
        };
        self.query(querier, req)
    }

    /// With metadata extension
    pub fn all_nft_info<Q: Querier, T: Into<String>, U: DeserializeOwned>(
        &self,
        querier: &Q,
        token_id: T,
        include_expired: bool,
    ) -> StdResult<AllNftInfoResponse<U>> {
        let req = Cw721QueryMsg::AllNftInfo {
            token_id: token_id.into(),
            include_expired: Some(include_expired),
        };
        self.query(querier, req)
    }

    /// With enumerable extension
    pub fn tokens<Q: Querier, T: Into<String>>(
        &self,
        querier: &Q,
        owner: T,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let req = Cw721QueryMsg::Tokens {
            owner: owner.into(),
            start_after,
            limit,
        };
        self.query(querier, req)
    }

    /// With enumerable extension
    pub fn all_tokens<Q: Querier>(
        &self,
        querier: &Q,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<TokensResponse> {
        let req = Cw721QueryMsg::AllTokens { start_after, limit };
        self.query(querier, req)
    }

    /// returns true if the contract supports the metadata extension
    pub fn has_metadata<Q: Querier>(&self, querier: &Q) -> bool {
        self.contract_info(querier).is_ok()
    }

    /// returns true if the contract supports the enumerable extension
    pub fn has_enumerable<Q: Querier>(&self, querier: &Q) -> bool {
        self.tokens(querier, self.addr(), None, Some(1)).is_ok()
    }
}
