use cosmwasm_schema::cw_serde;
use cosmwasm_std::Coin;

use std::{fmt, fmt::Display};

use cw_utils::NativeBalance;

use crate::Cw20CoinVerified;

#[cw_serde]

pub enum Balance {
    Native(NativeBalance),
    Cw20(Cw20CoinVerified),
}

impl Default for Balance {
    fn default() -> Balance {
        Balance::Native(NativeBalance(vec![]))
    }
}

impl Display for Balance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Balance::Native(native) => write!(f, "{native}"),
            Balance::Cw20(cw20) => write!(f, "{cw20}"),
        }?;
        Ok(())
    }
}

impl Balance {
    pub fn is_empty(&self) -> bool {
        match self {
            Balance::Native(balance) => balance.is_empty(),
            Balance::Cw20(coin) => coin.is_empty(),
        }
    }

    /// normalize Wallet
    pub fn normalize(&mut self) {
        match self {
            Balance::Native(balance) => balance.normalize(),
            Balance::Cw20(_) => {}
        }
    }
}

impl From<Vec<Coin>> for Balance {
    fn from(coins: Vec<Coin>) -> Balance {
        Balance::Native(NativeBalance(coins))
    }
}

impl From<Cw20CoinVerified> for Balance {
    fn from(cw20_coin: Cw20CoinVerified) -> Balance {
        Balance::Cw20(cw20_coin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{Addr, Uint128};

    #[test]
    fn default_balance_is_native() {
        let balance: Balance = Default::default();
        assert!(matches!(balance, Balance::Native(_)));
    }

    #[test]
    fn displaying_native_balance_works() {
        let balance: Balance = Default::default();
        assert_eq!("", format!("{balance}",));
    }

    #[test]
    fn displaying_cw20_balance_works() {
        let balance = Balance::Cw20(Cw20CoinVerified {
            address: Addr::unchecked("sender"),
            amount: Uint128::zero(),
        });
        assert_eq!("address: sender, amount: 0", format!("{balance}",));
    }

    #[test]
    fn default_native_balance_is_empty() {
        assert!(Balance::default().is_empty());
    }

    #[test]
    fn cw20_balance_with_zero_amount_is_empty() {
        assert!(Balance::Cw20(Cw20CoinVerified {
            address: Addr::unchecked("sender"),
            amount: Uint128::zero(),
        })
        .is_empty());
    }
}
