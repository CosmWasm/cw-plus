use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::NativeBalance;
use cw20::Cw20Coin;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Balance {
    Native(NativeBalance),
    Cw20(Cw20Coin),
}

impl Default for Balance {
    fn default() -> Balance {
        Balance::Native(NativeBalance(vec![]))
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
