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
        let mut v: Vec<u8> = addr.as_ref().into();
        v.push(0);
        v
    })
}

// set the end to the canonicalized format (used for Order::Descending)
pub fn calc_range_end(end_before: Option<Addr>) -> Option<Vec<u8>> {
    end_before.map(|addr| addr.as_ref().into())
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
    // TODO: add unit tests
    #[ignore]
    #[test]
    fn add_some_tests() {}
}
