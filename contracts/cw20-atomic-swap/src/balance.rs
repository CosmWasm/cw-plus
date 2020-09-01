use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, Coin, Uint128};

// TODO: Import from cw20-escrow
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20Coin {
    pub address: CanonicalAddr,
    pub amount: Uint128,
}

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
            Balance::Cw20(_) => false, // FIXME? zero amount coin
        }
    }
}
