use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Coin;
use cw20::Cw20Coin;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Balance {
    Native(Vec<Coin>),
    Cw20(Cw20Coin),
}

impl Default for Balance {
    fn default() -> Balance {
        Balance::Native(vec![])
    }
}

impl Balance {
    pub fn is_empty(&self) -> bool {
        match self {
            Balance::Native(coins) => coins.is_empty(),
            Balance::Cw20(coin) => coin.is_empty(),
        }
    }
}
