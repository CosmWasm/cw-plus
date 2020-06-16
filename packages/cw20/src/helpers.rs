use cosmwasm_std::{to_binary, HumanAddr, Querier, StdResult, WasmQuery};

use crate::{AllowanceResponse, Cw20QueryMsg, MetaResponse, MinterResponse};

/// ensure_cw20 checks that the contract satisfies the basic Cw20 query interface,
/// so we assume it is a valid contract. This can be used as a sanity check in init
/// when setting a token contract address.
///
/// This returns the token symbol if it is a valid contract.
pub fn ensure_cw20<Q: Querier>(querier: &Q, contract_addr: HumanAddr) -> StdResult<String> {
    let msg = Cw20QueryMsg::Meta {};
    let query = WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&msg)?,
    }
    .into();
    let res: MetaResponse = querier.query(&query)?;
    Ok(res.symbol)
}

/// ensure_cw20_allowance checks that the contract satisfies the basic Cw20 query interface
/// as well as the allowance extension. This can be used as a sanity check in init
/// when setting a token contract address.
///
/// This returns the token symbol if it is a valid contract.
pub fn ensure_cw20_allowance<Q: Querier>(
    querier: &Q,
    contract_addr: HumanAddr,
) -> StdResult<String> {
    // first ensure this is a valid erc20 contract
    let ticker = ensure_cw20(querier, contract_addr.clone())?;

    // we use the contract_addr here as we just want to ensure we have a valid HumanAddr, don't care what allowance
    let msg = Cw20QueryMsg::Allowance {
        owner: contract_addr.clone(),
        spender: contract_addr.clone(),
    };
    let query = WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&msg)?,
    }
    .into();
    // ensure we get a properly formatted AllowanceResponse, we don't care about the content
    let _: AllowanceResponse = querier.query(&query)?;

    Ok(ticker)
}

/// ensure_cw20_mintable checks that the contract satisfies the basic Cw20 query interface
/// as well as the mintable extension. This can be used as a sanity check in init
/// when setting a token contract address.
///
/// This returns the token symbol if it is a valid contract.
pub fn ensure_cw20_mintable<Q: Querier>(
    querier: &Q,
    contract_addr: HumanAddr,
) -> StdResult<String> {
    // first ensure this is a valid erc20 contract
    let ticker = ensure_cw20(querier, contract_addr.clone())?;

    let msg = Cw20QueryMsg::Minter {};
    let query = WasmQuery::Smart {
        contract_addr,
        msg: to_binary(&msg)?,
    }
    .into();
    // ensure we get a properly formatted AllowanceResponse, we don't care about the content
    let _: MinterResponse = querier.query(&query)?;

    Ok(ticker)
}
