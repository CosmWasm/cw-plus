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
pub fn calc_range_start_human(
    api: &dyn Api,
    start_after: Option<Addr>,
) -> StdResult<Option<Vec<u8>>> {
    match start_after {
        Some(human) => {
            let mut v: Vec<u8> = api.addr_canonicalize(human.as_ref())?.0.into();
            v.push(0);
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

// set the end to the canonicalized format (used for Order::Descending)
pub fn calc_range_end_human(api: &dyn Api, end_before: Option<Addr>) -> StdResult<Option<Vec<u8>>> {
    match end_before {
        Some(human) => {
            let v: Vec<u8> = api.addr_canonicalize(human.as_ref())?.into();
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

// this will set the first key after the provided key, by appending a 1 byte
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
