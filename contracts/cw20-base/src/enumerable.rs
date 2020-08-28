use cosmwasm_std::{
    Api, CanonicalAddr, Extern, HumanAddr, Order, Querier, ReadonlyStorage, StdResult, Storage,
};
use cw20::{AllAccountsResponse, AllAllowancesResponse, AllowanceInfo};

use crate::state::{allowances_read, balances_prefix_read};

// settings for pagination
const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_all_allowances<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    owner: HumanAddr,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<AllAllowancesResponse> {
    let owner_raw = deps.api.canonical_address(&owner)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);
    let api = &deps.api;

    let allowances: StdResult<Vec<AllowanceInfo>> = allowances_read(&deps.storage, &owner_raw)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (k, v) = item?;
            Ok(AllowanceInfo {
                spender: api.human_address(&CanonicalAddr::from(k))?,
                allowance: v.allowance,
                expires: v.expires,
            })
        })
        .collect();
    Ok(AllAllowancesResponse {
        allowances: allowances?,
    })
}

pub fn query_all_accounts<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    start_after: Option<HumanAddr>,
    limit: Option<u32>,
) -> StdResult<AllAccountsResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = calc_range_start(start_after);
    let api = &deps.api;

    let accounts: StdResult<Vec<_>> = balances_prefix_read(&deps.storage)
        .range(start.as_deref(), None, Order::Ascending)
        .take(limit)
        .map(|(k, _)| api.human_address(&CanonicalAddr::from(k)))
        .collect();

    Ok(AllAccountsResponse {
        accounts: accounts?,
    })
}

// this will set the first key after the provided key, by appending a 1 byte
fn calc_range_start(start_after: Option<HumanAddr>) -> Option<Vec<u8>> {
    start_after.map(|human| {
        let mut v = Vec::from(human.0);
        v.push(1);
        v
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{coins, Uint128};
    use cw20::{Expiration, TokenInfoResponse};

    use crate::contract::{handle, init, query_token_info};
    use crate::msg::{HandleMsg, InitMsg, InitialBalance};

    // this will set up the init for other tests
    fn do_init<S: Storage, A: Api, Q: Querier>(
        deps: &mut Extern<S, A, Q>,
        addr: &HumanAddr,
        amount: Uint128,
    ) -> TokenInfoResponse {
        let init_msg = InitMsg {
            name: "Auto Gen".to_string(),
            symbol: "AUTO".to_string(),
            decimals: 3,
            initial_balances: vec![InitialBalance {
                address: addr.into(),
                amount,
            }],
            mint: None,
        };
        let env = mock_env(&HumanAddr("creator".to_string()), &[]);
        init(deps, env, init_msg).unwrap();
        query_token_info(&deps).unwrap()
    }

    #[test]
    fn query_all_allowances_works() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        let owner = HumanAddr::from("owner");
        // these are in alphabetical order different than insert order
        let spender1 = HumanAddr::from("later");
        let spender2 = HumanAddr::from("earlier");

        let env = mock_env(owner.clone(), &[]);
        do_init(&mut deps, &owner, Uint128(12340000));

        // no allowance to start
        let allowances = query_all_allowances(&deps, owner.clone(), None, None).unwrap();
        assert_eq!(allowances.allowances, vec![]);

        // set allowance with height expiration
        let allow1 = Uint128(7777);
        let expires = Expiration::AtHeight(5432);
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender1.clone(),
            amount: allow1,
            expires: Some(expires.clone()),
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // set allowance with no expiration
        let allow2 = Uint128(54321);
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow2,
            expires: None,
        };
        handle(&mut deps, env.clone(), msg).unwrap();

        // query list gets 2
        let allowances = query_all_allowances(&deps, owner.clone(), None, None).unwrap();
        assert_eq!(allowances.allowances.len(), 2);

        // first one is spender2 ("earlier")
        let allowances = query_all_allowances(&deps, owner.clone(), None, Some(1)).unwrap();
        assert_eq!(allowances.allowances.len(), 1);
        let allow = &allowances.allowances[0];
        assert_eq!(&allow.spender, &spender2);
        assert_eq!(&allow.expires, &Expiration::Never {});
        assert_eq!(&allow.allowance, &allow2);

        // next one is spender1 ("later")
        let allowances =
            query_all_allowances(&deps, owner.clone(), Some(spender2), Some(10000)).unwrap();
        assert_eq!(allowances.allowances.len(), 1);
        let allow = &allowances.allowances[0];
        assert_eq!(&allow.spender, &spender1);
        assert_eq!(&allow.expires, &expires);
        assert_eq!(&allow.allowance, &allow1);
    }

    #[test]
    fn query_all_accounts_works() {
        let mut deps = mock_dependencies(20, &coins(2, "token"));

        // insert order and lexographical order are different
        let acct1 = HumanAddr::from("acct01");
        let acct2 = HumanAddr::from("zebra");
        let acct3 = HumanAddr::from("nice");
        let acct4 = HumanAddr::from("aaaardvark");
        let expected_order = [acct4.clone(), acct1.clone(), acct3.clone(), acct2.clone()];

        do_init(&mut deps, &acct1, Uint128(12340000));

        // put money everywhere (to create balanaces)
        let env = mock_env(acct1.clone(), &[]);
        handle(
            &mut deps,
            env.clone(),
            HandleMsg::Transfer {
                recipient: acct2,
                amount: Uint128(222222),
            },
        )
        .unwrap();
        handle(
            &mut deps,
            env.clone(),
            HandleMsg::Transfer {
                recipient: acct3,
                amount: Uint128(333333),
            },
        )
        .unwrap();
        handle(
            &mut deps,
            env.clone(),
            HandleMsg::Transfer {
                recipient: acct4,
                amount: Uint128(444444),
            },
        )
        .unwrap();

        // make sure we get the proper results
        let accounts = query_all_accounts(&deps, None, None).unwrap();
        assert_eq!(accounts.accounts, expected_order.clone());

        // let's do pagination
        let accounts = query_all_accounts(&deps, None, Some(2)).unwrap();
        assert_eq!(accounts.accounts, expected_order[0..2].to_vec());

        let accounts =
            query_all_accounts(&deps, Some(accounts.accounts[1].clone()), Some(1)).unwrap();
        assert_eq!(accounts.accounts, expected_order[2..3].to_vec());

        let accounts =
            query_all_accounts(&deps, Some(accounts.accounts[0].clone()), Some(777)).unwrap();
        assert_eq!(accounts.accounts, expected_order[3..].to_vec());
    }
}
