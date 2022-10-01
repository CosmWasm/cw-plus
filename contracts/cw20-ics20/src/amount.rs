use crate::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{testing::MockApi, Api, Coin, Uint128};
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
                address: MockApi::default()
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
            address: MockApi::default().addr_validate(addr).unwrap(),
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
