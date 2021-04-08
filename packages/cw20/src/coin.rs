use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Cw20Coin {
    pub address: Addr,
    pub amount: Uint128,
}

impl Cw20Coin {
    pub fn is_empty(&self) -> bool {
        self.amount == Uint128(0)
    }
}
