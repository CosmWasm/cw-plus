use cosmwasm_std::{
    Api, CanonicalAddr, Extern, HumanAddr, Order, Querier, ReadonlyStorage, StdResult, Storage,
};
use cw0::calc_range_start_human;
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
    let start = calc_range_start_human(deps.api, start_after)?;
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
    let start = calc_range_start_human(deps.api, start_after)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Uint128, MessageInfo, Env, Coin};
    use cw20::{Cw20CoinHuman, Expiration, TokenInfoResponse};

    use crate::contract::{handle, init, query_token_info};
    use crate::msg::{HandleMsg, InitMsg};

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
            initial_balances: vec![Cw20CoinHuman {
                address: addr.into(),
                amount,
            }],
            mint: None,
        };
        let (env,info) = mock_env_info(&HumanAddr("creator".to_string()), &[]);
        init(deps, env, info,init_msg).unwrap();
        query_token_info(&deps).unwrap()
    }

    fn mock_env_info<U: Into<HumanAddr>>(sender: U, sent: &[Coin]) -> (Env, MessageInfo) {
        (mock_env(), mock_info(sender, sent))
    }

    #[test]
    fn query_all_allowances_works() {
        let mut deps = mock_dependencies(&coins(2, "token"));

        let owner = HumanAddr::from("owner");
        // these are in alphabetical order different than insert order
        let spender1 = HumanAddr::from("later");
        let spender2 = HumanAddr::from("earlier");

        let (env,info) = mock_env_info(owner.clone(), &[]);
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
        handle(&mut deps, env.clone(), info.clone(), msg).unwrap();

        // set allowance with no expiration
        let allow2 = Uint128(54321);
        let msg = HandleMsg::IncreaseAllowance {
            spender: spender2.clone(),
            amount: allow2,
            expires: None,
        };
        handle(&mut deps, env.clone(),info.clone(), msg).unwrap();

        // query list gets 2
        let allowances = query_all_allowances(&deps, owner.clone(), None, None).unwrap();
        assert_eq!(allowances.allowances.len(), 2);

        /*
        // first one is spender2 ("earlier")
        let allowances = query_all_allowances(&deps, owner.clone(), None, Some(1)).unwrap();
        assert_eq!(allowances.allowances.len(), 1);
        let allow = &allowances.allowances[0];
        assert_eq!(&allow.spender, &spender2);
        assert_eq!(&allow.expires, &Expiration::Never {});
        assert_eq!(&allow.allowance, &allow2);

         */

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
        let mut deps = mock_dependencies(&coins(2, "token"));

        // insert order and lexographical order are different
        let acct1 = HumanAddr::from("acct01");
        let acct2 = HumanAddr::from("zebra");
        let acct3 = HumanAddr::from("nice");
        let acct4 = HumanAddr::from("aaaardvark");
        let expected_order = [acct2.clone(), acct1.clone(), acct3.clone(), acct4.clone()];

        do_init(&mut deps, &acct1, Uint128(12340000));

        // put money everywhere (to create balanaces)
        let (env,info) = mock_env_info(acct1.clone(), &[]);
        handle(
            &mut deps,
            env.clone(),
            info.clone(),HandleMsg::Transfer {
                recipient: acct2,
                amount: Uint128(222222),
            },
        )
        .unwrap();
        handle(
            &mut deps,
            env.clone(),
            info.clone(),
            HandleMsg::Transfer {
                recipient: acct3,
                amount: Uint128(333333),
            },
        )
        .unwrap();
         handle(
            &mut deps,
            env.clone(),
            info.clone(),
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
