use crate::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, CanonicalAddr, Coin, StdError, StdResult, Uint128};
use cw20::Cw20CoinVerified;
use std::convert::TryInto;

#[cw_serde]
pub enum Amount {
    Native(Coin),
    Cw20(Cw20CoinVerified),
}

impl Amount {
    pub fn from_parts(denom: String, amount: Uint128) -> Self {
        if denom.starts_with("cw20:") {
            Amount::Cw20(Cw20CoinVerified {
                address: LocalApi::default()
                    .addr_validate(denom.get(5..).unwrap())
                    .unwrap(),
                amount,
            })
        } else {
            Amount::Native(Coin { denom, amount })
        }
    }

    pub fn cw20(amount: u128, addr: &str) -> Self {
        Amount::Cw20(Cw20CoinVerified {
            address: LocalApi::default().addr_validate(addr).unwrap(),
            amount: Uint128::new(amount),
        })
    }

    pub fn native(amount: u128, denom: &str) -> Self {
        Amount::Native(Coin {
            denom: denom.to_string(),
            amount: Uint128::new(amount),
        })
    }
}

impl Amount {
    pub fn denom(&self) -> String {
        match self {
            Amount::Native(c) => c.denom.clone(),
            Amount::Cw20(c) => format!("cw20:{}", c.address.as_str()),
        }
    }

    pub fn amount(&self) -> Uint128 {
        match self {
            Amount::Native(c) => c.amount,
            Amount::Cw20(c) => c.amount,
        }
    }

    /// convert the amount into u64
    pub fn u64_amount(&self) -> Result<u64, ContractError> {
        Ok(self.amount().u128().try_into()?)
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Amount::Native(c) => c.amount.is_zero(),
            Amount::Cw20(c) => c.amount.is_zero(),
        }
    }
}

// This simple Api provided for address verification
// It based on MockApi
// https://github.com/CosmWasm/cosmwasm/blob/main/packages/std/src/testing/mock.rs
#[derive(Copy, Clone)]
struct LocalApi {
    length_min: usize,
    length_max: usize,
    shuffles_encode: usize,
    shuffles_decode: usize,
}

impl Default for LocalApi {
    fn default() -> Self {
        LocalApi {
            length_min: 3,
            length_max: 54,
            shuffles_encode: 18,
            shuffles_decode: 2,
        }
    }
}

impl LocalApi {
    fn digit_sum(input: &[u8]) -> usize {
        input.iter().fold(0, |sum, val| sum + (*val as usize))
    }

    pub fn riffle_shuffle<T: Clone>(input: &[T]) -> Vec<T> {
        assert!(
            input.len() % 2 == 0,
            "Method only defined for even number of elements"
        );
        let mid = input.len() / 2;
        let (left, right) = input.split_at(mid);
        let mut out = Vec::<T>::with_capacity(input.len());
        for i in 0..mid {
            out.push(right[i].clone());
            out.push(left[i].clone());
        }
        out
    }

    fn addr_canonicalize(&self, input: &str) -> StdResult<CanonicalAddr> {
        let api = Self::default();

        // Dummy input validation. This is more sophisticated for formats like bech32, where format and checksum are validated.
        if input.len() < api.length_min {
            return Err(StdError::generic_err(
                "Invalid input: human address too short",
            ));
        }
        if input.len() > api.length_max {
            return Err(StdError::generic_err(
                "Invalid input: human address too long",
            ));
        }

        // mimicks formats like hex or bech32 where different casings are valid for one address
        let normalized = input.to_lowercase();

        let mut out = Vec::from(normalized);

        // pad to canonical length with NULL bytes
        out.resize(api.length_max, 0x00);
        // content-dependent rotate followed by shuffle to destroy
        // the most obvious structure (https://github.com/CosmWasm/cosmwasm/issues/552)
        let rotate_by = Self::digit_sum(&out) % api.length_max;
        out.rotate_left(rotate_by);
        for _ in 0..api.shuffles_encode {
            out = Self::riffle_shuffle(&out);
        }
        Ok(out.into())
    }

    fn addr_humanize(&self, canonical: &CanonicalAddr) -> StdResult<Addr> {
        let api = Self::default();

        if canonical.len() != api.length_max {
            return Err(StdError::generic_err(
                "Invalid input: canonical address length not correct",
            ));
        }

        let mut tmp: Vec<u8> = canonical.clone().into();
        // Shuffle two more times which restored the original value (24 elements are back to original after 20 rounds)
        for _ in 0..api.shuffles_decode {
            tmp = Self::riffle_shuffle(&tmp);
        }
        // Rotate back
        let rotate_by = Self::digit_sum(&tmp) % api.length_max;
        tmp.rotate_right(rotate_by);
        // Remove NULL bytes (i.e. the padding)
        let trimmed = tmp.into_iter().filter(|&x| x != 0x00).collect();
        // decode UTF-8 bytes into string
        let human = String::from_utf8(trimmed)?;
        Ok(Addr::unchecked(human))
    }

    fn addr_validate(&self, input: &str) -> StdResult<Addr> {
        let canonical = self.addr_canonicalize(input)?;
        let normalized = self.addr_humanize(&canonical)?;
        if input != normalized {
            return Err(StdError::generic_err(
                "Invalid input: address not normalized",
            ));
        }

        Ok(Addr::unchecked(input))
    }
}

#[cfg(test)]
mod test {
    use super::{Amount, Coin, Cw20CoinVerified, Uint128};
    use cosmwasm_std::Addr;

    #[test]
    fn from_parts_native() {
        let amount = Uint128::new(1_000_000);
        let denom = "token-denom";

        assert_eq!(
            Amount::from_parts(denom.into(), amount),
            Amount::Native(Coin {
                denom: denom.into(),
                amount
            })
        );
    }

    #[test]
    fn from_parts_cw20() {
        let amount = Uint128::new(1_000_000);
        let cw20_addr = Addr::unchecked("token-addr");
        let cw20_denom = "cw20:token-addr";

        assert_eq!(
            Amount::from_parts(cw20_denom.into(), amount),
            Amount::Cw20(Cw20CoinVerified {
                address: cw20_addr,
                amount
            })
        );
    }

    #[test]
    #[should_panic]
    fn from_parts_cw20_bad_addr() {
        let amount = Uint128::new(1_000_000);
        let cw20_denom = "cw20:BAD-token-addr";

        Amount::from_parts(cw20_denom.into(), amount);
    }
}
