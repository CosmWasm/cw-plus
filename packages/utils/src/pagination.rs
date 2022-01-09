use cosmwasm_std::{Addr, Api, CanonicalAddr, StdResult};

// this is used for pagination. Maybe we move it into the std lib one day?
pub fn maybe_canonical(api: &dyn Api, human: Option<Addr>) -> StdResult<Option<CanonicalAddr>> {
    human.map(|x| api.addr_canonicalize(x.as_ref())).transpose()
}

// This is used for pagination. Maybe we move it into the std lib one day?
pub fn maybe_addr(api: &dyn Api, human: Option<String>) -> StdResult<Option<Addr>> {
    human.map(|x| api.addr_validate(&x)).transpose()
}

// this will set the first key after the provided key, by appending a 0 byte
pub fn calc_range_start(start_after: Option<Addr>) -> Option<Vec<u8>> {
    start_after.map(|addr| {
        let mut v: Vec<u8> = addr.as_bytes().into();
        v.push(0);
        v
    })
}

// set the end to the canonicalized format (used for Order::Descending)
pub fn calc_range_end(end_before: Option<Addr>) -> Option<Vec<u8>> {
    end_before.map(|addr| addr.as_bytes().into())
}

// this will set the first key after the provided key, by appending a 0 byte
pub fn calc_range_start_string(start_after: Option<String>) -> Option<Vec<u8>> {
    start_after.map(|token_id| {
        let mut v: Vec<u8> = token_id.into_bytes();
        v.push(0);
        v
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_std::{testing::mock_dependencies, Order};
    use cw_storage_plus::{Bound, Map};

    pub const HOLDERS: Map<&Addr, usize> = Map::new("some_data");
    const LIMIT: usize = 30;

    fn addr_from_i(i: usize) -> Addr {
        Addr::unchecked(format!("addr{:0>8}", i))
    }

    #[test]
    fn calc_range_start_works_as_expected() {
        let total_elements_count = 100;
        let mut deps = mock_dependencies();
        for i in 0..total_elements_count {
            let holder = (addr_from_i(i), i);
            HOLDERS
                .save(&mut deps.storage, &holder.0, &holder.1)
                .unwrap();
        }

        for j in 0..4 {
            let start_after = if j == 0 {
                None
            } else {
                Some(addr_from_i(j * LIMIT - 1))
            };

            let start = calc_range_start(start_after).map(Bound::ExclusiveRaw);

            let holders = HOLDERS
                .keys(&deps.storage, start, None, Order::Ascending)
                .take(LIMIT)
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            for (i, holder) in holders.into_iter().enumerate() {
                let global_index = j * LIMIT + i;
                assert_eq!(holder, addr_from_i(global_index));
            }
        }
    }

    #[test]
    fn calc_range_end_works_as_expected() {
        let total_elements_count = 100;
        let mut deps = mock_dependencies();
        for i in 0..total_elements_count {
            let holder = (addr_from_i(i), i);
            HOLDERS
                .save(&mut deps.storage, &holder.0, &holder.1)
                .unwrap();
        }

        for j in 0..4 {
            let end_before = Some(addr_from_i(total_elements_count - j * LIMIT));

            let end = calc_range_end(end_before).map(Bound::ExclusiveRaw);

            let holders = HOLDERS
                .keys(&deps.storage, None, end, Order::Descending)
                .take(LIMIT)
                .collect::<StdResult<Vec<_>>>()
                .unwrap();

            for (i, holder) in holders.into_iter().enumerate() {
                let global_index = total_elements_count - i - j * LIMIT - 1;
                assert_eq!(holder, addr_from_i(global_index));
            }
        }
    }

    // TODO: add unit tests
    #[ignore]
    #[test]
    fn add_more_tests() {}
}
