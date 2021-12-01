use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20Coin {
    pub address: String,
    pub amount: Uint128,
}

impl Cw20Coin {
    pub fn is_empty(&self) -> bool {
        self.amount == Uint128::zero()
    }
}

impl fmt::Display for Cw20Coin {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "address: {}, amount: {}", self.address, self.amount)
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20CoinVerified {
    pub address: Addr,
    pub amount: Uint128,
}

impl Cw20CoinVerified {
    pub fn is_empty(&self) -> bool {
        self.amount == Uint128::zero()
    }
}

impl fmt::Display for Cw20CoinVerified {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "address: {}, amount: {}", self.address, self.amount)
    }
}
