use cosmwasm_std::CanonicalAddr;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Denom {
    Native(String),
    Cw20(CanonicalAddr),
}

impl Default for Denom {
    fn default() -> Denom {
        Denom::Native(String::default())
    }
}

impl Denom {
    pub fn is_empty(&self) -> bool {
        match self {
            Denom::Native(string) => string.is_empty(),
            Denom::Cw20(canonical_addr) => canonical_addr.is_empty(),
        }
    }
}

