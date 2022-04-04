use std::convert::TryInto;
use cosmwasm_std::{Addr, BlockInfo, Deps, Order, StdError, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Key, Map, Prefix, PrefixBound, PrimaryKey, CwIntKey, Bound};
use cw_utils::{Expiration, Scheduled};
use itertools::FoldWhile::{Continue, Done};
use itertools::{FoldWhile, Itertools};

pub struct IndexedClaim<'a> {
    // claims stores (addr, timestamp) -> claim
    pub claims: Map<'a, (&'a Addr, u64), Uint128>,
    // if max index is set, defined number of claims will be indexed on the storage
    pub max_index: Option<u64>,
    pub index_size: Item<'a, u64>,
    pub total_claims: Map<'a, &'a Addr, Uint128>,
    pub total_claim_sort_index: Map<'a, (u128, &'a Addr), bool>
}

impl<'a> IndexedClaim<'a> {
    pub const fn new(storage_key: &'a str, max_index: Option<u64>) -> Self {

        // TODO: fix storage key concat
        IndexedClaim {
            claims: Map::new(storage_key),
            total_claims: Map::new("_total_claims"),
            total_claim_sort_index: Map::new("_total_claims_index"),
            index_size: Item::new("_index_size"),
            max_index,
        }
    }

    /// This creates a claim, such that the given address can claim an amount of tokens after
    /// the release date.
    pub fn create_claim(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        amount: Uint128,
        release_at: Timestamp,
    ) -> StdResult<()> {
        self.claims.save(storage, (addr, release_at.nanos()), &amount)?;

        // if max index is setup, save the
        if let Some(max_index) = self.max_index {
            // increase users total claim
            let total_claim = self.total_claims.update(storage, addr, |old| -> StdResult<_> {
                match old {
                    None => Ok(amount),
                    Some(o) => Ok(amount + o)
                }
            })?;

            let mut current_index_size = self.index_size.may_load(storage)?.unwrap_or(0);

            let prev_claim = self.total_claim_sort_index
                // TODO: maybe implement keys_prefix?
                .prefix_range(storage, Some(PrefixBound::exclusive((total_claim.u128().to_cw_bytes().as_slice()))), None, Order::Ascending)
                .map(|r| r.map(|((amount, addr), v)| (amount, addr)))
                .take(1 as usize)
                .collect::<StdResult<Vec<_>>>()?;

            let (raw_prev_claim_amount, _) = prev_claim.first().unwrap();

            let prev_claim_amount = CwIntKey::from_cw_bytes(raw_prev_claim_amount);
            if total_claim > prev_claim_amount {
                // save indexes
                current_index_size += 1;
                self.index_size.save(storage, &current_index_size)?;
                self.total_claim_sort_index.save(storage, (&amount.u128().to_cw_bytes(), &addr), &true)?;

            }

            if current_index_size >= max_index {
                let last_claim = self.total_claim_sort_index
                    // TODO: maybe implement keys_prefix?
                    .prefix_range(storage, Some(PrefixBound::exclusive((total_claim.u128().to_cw_bytes().as_slice()))), None, Order::Ascending)
                    .map(|r| r.map(|((amount, addr), v)| (amount, addr)))
                    .take(1 as usize)
                    .collect::<StdResult<Vec<_>>>()?;

                let (raw_last_claim_amount, last_claim_addr) = last_claim.first().unwrap();
                self.total_claim_sort_index.remove(storage, (prev_claim_amount, last_claim_addr))
            }

        }

        Ok(())
    }

    /// This iterates over all mature claims for the address, and removes them, up to an optional cap.
    /// it removes the finished claims and returns the total amount of tokens to be released.
    pub fn claim_tokens(
        &self,
        storage: &mut dyn Storage,
        addr: &Addr,
        block: &BlockInfo,
        cap: Option<Uint128>,
    ) -> StdResult<Uint128> {
        let claims: Vec<_> =
            self.claims
                .prefix(addr)
                .range(storage, None, Some(Bound::inclusive(block.time.nanos())), Order::Descending)
                .collect::<StdResult<_>>()?;

        let claim_amount = claims.into_iter()
            .fold_while(Uint128::zero(), |acc, (timestamp, claim)| {
                // TODO: handle partial paying claims?
                let to_send = acc + claim;
                if let Some(cap) = cap {
                    if to_send > cap {
                        return Done(acc)
                    }}
                self.claims.remove(storage, (&addr, timestamp));
                Continue(to_send)
            }).into_inner();

        if let Some(max_index) = self.max_index {

            let total_claims = self.total_claims.load(storage, addr)?;
            let reduced_claims = total_claims - claim_amount;

            self.total_claims.save(storage, addr, &reduced_claims)?;

            let total_claim_raw = total_claims.u128().to_cw_bytes();
            self.total_claim_sort_index.remove(storage, (total_claim_raw.as_slice(), addr));
        }
        Ok(claim_amount)
    }
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{
        testing::{mock_dependencies, mock_env},
        Order,
    };

    use super::*;
    const TEST_AMOUNT: u128 = 1000u128;
    const TEST_EXPIRATION: Expiration = Expiration::AtHeight(10);

    #[test]
    fn can_create_claims() {
        let deps = mock_dependencies();
        let claims = IndexedClaim::new("claims", None);
        // Assert that claims creates a map and there are no keys in the map.
        assert_eq!(
            claims
                .claims
                .range_raw(&deps.storage, None, None, Order::Ascending)
                .collect::<StdResult<Vec<_>>>()
                .unwrap()
                .len(),
            0
        );
    }

    /*
    #[test]
    fn check_create_claim_updates_map() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                TEST_AMOUNT.into(),
                TEST_EXPIRATION,
            )
            .unwrap();

        // Assert that claims creates a map and there is one claim for the address.
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(saved_claims.len(), 1);
        assert_eq!(saved_claims[0].amount, TEST_AMOUNT.into());
        assert_eq!(saved_claims[0].release_at, TEST_EXPIRATION);

        // Adding another claim to same address, make sure that both claims are saved.
        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                TEST_EXPIRATION,
            )
            .unwrap();

        // Assert that both claims exist for the address.
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(saved_claims[0].amount, TEST_AMOUNT.into());
        assert_eq!(saved_claims[0].release_at, TEST_EXPIRATION);
        assert_eq!(saved_claims[1].amount, (TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims[1].release_at, TEST_EXPIRATION);

        // Adding another claim to different address, make sure that other address only has one claim.
        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr2"),
                (TEST_AMOUNT + 100).into(),
                TEST_EXPIRATION,
            )
            .unwrap();

        // Assert that both claims exist for the address.
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        let saved_claims_addr2 = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr2"))
            .unwrap();
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(saved_claims_addr2.len(), 1);
    }

    #[test]
    fn test_claim_tokens_with_no_claims() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &mock_env().block,
                None,
            )
            .unwrap();
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(amount, Uint128::zero());
        assert_eq!(saved_claims.len(), 0);
    }

    #[test]
    fn test_claim_tokens_with_no_released_claims() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 0;
        // the address has two claims however they are both not expired
        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &env.block,
                None,
            )
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(amount, Uint128::zero());
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(saved_claims[0].amount, (TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims[0].release_at, Expiration::AtHeight(10));
        assert_eq!(saved_claims[1].amount, (TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims[1].release_at, Expiration::AtHeight(100));
    }

    #[test]
    fn test_claim_tokens_with_one_released_claim() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                TEST_AMOUNT.into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 20;
        // the address has two claims and the first one can be released
        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &env.block,
                None,
            )
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(amount, TEST_AMOUNT.into());
        assert_eq!(saved_claims.len(), 1);
        assert_eq!(saved_claims[0].amount, (TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims[0].release_at, Expiration::AtHeight(100));
    }

    #[test]
    fn test_claim_tokens_with_all_released_claims() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                TEST_AMOUNT.into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 1000;
        // the address has two claims and both can be released
        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &env.block,
                None,
            )
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(amount, (TEST_AMOUNT + TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims.len(), 0);
    }

    #[test]
    fn test_claim_tokens_with_zero_cap() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                TEST_AMOUNT.into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 1000;

        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &env.block,
                Some(Uint128::zero()),
            )
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(amount, Uint128::zero());
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(saved_claims[0].amount, (TEST_AMOUNT).into());
        assert_eq!(saved_claims[0].release_at, Expiration::AtHeight(10));
        assert_eq!(saved_claims[1].amount, (TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims[1].release_at, Expiration::AtHeight(100));
    }

    #[test]
    fn test_claim_tokens_with_cap_greater_than_pending_claims() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                TEST_AMOUNT.into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(100),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 1000;

        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &env.block,
                Some(Uint128::from(2100u128)),
            )
            .unwrap();

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();

        assert_eq!(amount, (TEST_AMOUNT + TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims.len(), 0);
    }

    #[test]
    fn test_claim_tokens_with_cap_only_one_claim_released() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                TEST_AMOUNT.into(),
                Expiration::AtHeight(5),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 1000;
        // the address has two claims and the first one can be released
        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &env.block,
                Some((TEST_AMOUNT + 50).into()),
            )
            .unwrap();
        assert_eq!(amount, (TEST_AMOUNT).into());

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(saved_claims.len(), 1);
        assert_eq!(saved_claims[0].amount, (TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims[0].release_at, Expiration::AtHeight(10));
    }

    #[test]
    fn test_claim_tokens_with_cap_too_low_no_claims_released() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                TEST_AMOUNT.into(),
                Expiration::AtHeight(5),
            )
            .unwrap();

        let mut env = mock_env();
        env.block.height = 1000;
        // the address has two claims and the first one can be released
        let amount = claims
            .claim_tokens(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                &env.block,
                Some((TEST_AMOUNT - 50).into()),
            )
            .unwrap();
        assert_eq!(amount, Uint128::zero());

        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(saved_claims.len(), 2);
        assert_eq!(saved_claims[0].amount, (TEST_AMOUNT + 100).into());
        assert_eq!(saved_claims[0].release_at, Expiration::AtHeight(10));
        assert_eq!(saved_claims[1].amount, (TEST_AMOUNT).into());
        assert_eq!(saved_claims[1].release_at, Expiration::AtHeight(5));
    }

    #[test]
    fn test_query_claims_returns_correct_claims() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        let queried_claims = claims
            .query_claims(deps.as_ref(), &Addr::unchecked("addr"))
            .unwrap();
        let saved_claims = claims
            .0
            .load(deps.as_mut().storage, &Addr::unchecked("addr"))
            .unwrap();
        assert_eq!(queried_claims.claims, saved_claims);
    }

    #[test]
    fn test_query_claims_returns_empty_for_non_existent_user() {
        let mut deps = mock_dependencies();
        let claims = Claims::new("claims");

        claims
            .create_claim(
                deps.as_mut().storage,
                &Addr::unchecked("addr"),
                (TEST_AMOUNT + 100).into(),
                Expiration::AtHeight(10),
            )
            .unwrap();

        let queried_claims = claims
            .query_claims(deps.as_ref(), &Addr::unchecked("addr2"))
            .unwrap();

        assert_eq!(queried_claims.claims.len(), 0);
    }

     */
}
