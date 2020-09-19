use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, GenericCoin, HumanAddr, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20Coin {
    pub address: CanonicalAddr,
    pub amount: Uint128,
}

impl Cw20Coin {
    pub fn is_empty(&self) -> bool {
        self.amount == Uint128(0)
    }
}

impl GenericCoin for Cw20Coin {
    fn key(&self) -> String {
        self.address.to_string()
    }

    fn value(&self) -> Uint128 {
        self.amount
    }

    fn add_value(&mut self, add_value: Uint128) {
        self.amount += add_value;
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20CoinHuman {
    pub address: HumanAddr,
    pub amount: Uint128,
}
