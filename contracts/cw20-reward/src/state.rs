use cosmwasm_std::{Addr, Api, CanonicalAddr, Decimal, Deps, Order, StdResult, Storage, Uint128};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw_storage_plus::{Bound, Item, Map};

pub const STATE: Item<State> = Item::new("\u{0}\u{5}state");
pub const CONFIG: Item<Config> = Item::new("\u{0}\u{6}config");
pub const HOLDERS: Map<&[u8], Holder> = Map::new("holders");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub hub_contract: CanonicalAddr,
    pub reward_denom: String,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub global_index: Decimal,
    pub total_balance: Uint128,
    pub prev_reward_balance: Uint128,
}

pub fn store_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    STATE.save(storage, state)
}

pub fn read_state(storage: &dyn Storage) -> StdResult<State> {
    STATE.load(storage)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Holder {
    pub balance: Uint128,
    pub index: Decimal,
    pub pending_rewards: Decimal,
}

// This is similar to HashMap<holder's address, Hodler>
pub fn store_holder(
    storage: &mut dyn Storage,
    holder_address: &CanonicalAddr,
    holder: &Holder,
) -> StdResult<()> {
    HOLDERS.save(storage, holder_address.as_slice(), holder)
}

pub fn read_holder(storage: &dyn Storage, holder_address: &CanonicalAddr) -> StdResult<Holder> {
    let res = HOLDERS.may_load(storage, holder_address.as_slice())?;
    match res {
        Some(holder) => Ok(holder),
        None => Ok(Holder {
            balance: Uint128::zero(),
            index: Decimal::zero(),
            pending_rewards: Decimal::zero(),
        }),
    }
}

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_holders(
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
            let address: String = deps.api.addr_humanize(&CanonicalAddr::from(k))?.to_string();
            Ok(HolderResponse {
                address,
                balance: v.balance,
                index: v.index,
                pending_rewards: v.pending_rewards,
            })
        })
        .collect()
}

// this will set the first key after the provided key, by appending a 1 byte
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

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::{Api, StdResult, Storage};
    use cosmwasm_storage::{
        bucket, bucket_read, singleton, singleton_read, Bucket, ReadonlyBucket,
    };

    pub static STATE_KEY: &[u8] = b"state";
    pub static CONFIG_KEY: &[u8] = b"config";
    pub static PREFIX_HOLDERS: &[u8] = b"holders";

    pub fn store_state(storage: &mut dyn Storage, params: &State) -> StdResult<()> {
        singleton(storage, STATE_KEY).save(params)
    }
    pub fn read_state(storage: &dyn Storage) -> StdResult<State> {
        singleton_read(storage, STATE_KEY).load()
    }

    pub fn store_legacy_config(storage: &mut dyn Storage, params: &Config) -> StdResult<()> {
        singleton(storage, CONFIG_KEY).save(params)
    }
    pub fn read_legacy_config(storage: &dyn Storage) -> StdResult<Config> {
        singleton_read(storage, CONFIG_KEY).load()
    }

    /// balances are state of the erc20 tokens
    pub fn legacy_holders(storage: &mut dyn Storage) -> Bucket<Holder> {
        bucket(storage, PREFIX_HOLDERS)
    }

    /// balances are state of the erc20 tokens (read-only version for queries)
    pub fn legacy_holders_read(storage: &dyn Storage) -> ReadonlyBucket<Holder> {
        bucket_read(storage, PREFIX_HOLDERS)
    }

    #[test]
    fn state_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        store_state(
            &mut deps.storage,
            &State {
                global_index: Default::default(),
                total_balance: Default::default(),
                prev_reward_balance: Default::default(),
            },
        )
        .unwrap();

        assert_eq!(
            STATE.load(&deps.storage).unwrap(),
            read_state(&deps.storage).unwrap()
        );
    }

    #[test]
    fn config_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        store_legacy_config(
            &mut deps.storage,
            &Config {
                hub_contract: deps.api.addr_canonicalize("hub").unwrap(),
                reward_denom: "".to_string(),
            },
        )
        .unwrap();

        assert_eq!(
            CONFIG.load(&deps.storage).unwrap(),
            read_legacy_config(&deps.storage).unwrap()
        );
    }

    #[test]
    fn holders_legacy_compatibility() {
        let mut deps = mock_dependencies(&[]);
        let mut balances = legacy_holders(&mut deps.storage);
        let addr1 = deps.api.addr_canonicalize("addr0000").unwrap();
        let key1 = addr1.as_slice();

        balances
            .save(
                key1,
                &Holder {
                    balance: Uint128::from(200u128),
                    index: Default::default(),
                    pending_rewards: Default::default(),
                },
            )
            .unwrap();

        let balances_read = legacy_holders_read(&deps.storage);
        assert_eq!(
            HOLDERS.load(&deps.storage, key1).unwrap(),
            balances_read.load(key1).unwrap()
        );
    }
}
