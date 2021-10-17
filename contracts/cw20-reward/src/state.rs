use crate::msg::HolderResponse;
use cosmwasm_std::{Addr, Api, CanonicalAddr, Decimal, Deps, Order, StdResult, Uint128};
use cw_controllers::Claims;
use cw_storage_plus::{Bound, Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub cw20_token_addr: String,
    pub unbonding_period: u64,
    pub global_index: Decimal,
    pub total_balance: Uint128,
    pub prev_reward_balance: Uint128,
}
pub const STATE: Item<State> = Item::new("state");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Holder {
    pub balance: Uint128,
    pub index: Decimal,
    pub pending_rewards: Decimal,
}

// REWARDS (holder_addr, cw20_addr) -> Holder
pub const HOLDERS: Map<&Addr, Holder> = Map::new("holders");

pub const CLAIMS: Claims = Claims::new("claims");

/// list_accrued_rewards settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn list_accrued_rewards(
    deps: Deps,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> StdResult<Vec<HolderResponse>> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(deps.api, start_after.map(Addr::unchecked))?.map(Bound::exclusive);

    HOLDERS
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|elem| {
            let (k, v) = elem?;
            let address = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            Ok(HolderResponse {
                address,
                balance: v.balance,
                index: v.index,
                pending_rewards: v.pending_rewards,
            })
        })
        .collect()
}

fn calc_range_start(api: &dyn Api, start_after: Option<Addr>) -> StdResult<Option<Vec<u8>>> {
    match start_after {
        Some(human) => {
            let mut v: Vec<u8> = api.addr_canonicalize(human.as_ref())?.0.into();
            v.push(0);
            Ok(Some(v))
        }
        None => Ok(None),
    }
}
