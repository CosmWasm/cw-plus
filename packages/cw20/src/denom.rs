use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]

pub enum Denom {
    Native(String),
    Cw20(Addr),
}

// TODO: remove or figure out where needed
impl Default for Denom {
    fn default() -> Denom {
        Denom::Native(String::default())
    }
}

impl Denom {
    pub fn is_empty(&self) -> bool {
        match self {
            Denom::Native(string) => string.is_empty(),
            Denom::Cw20(addr) => addr.as_ref().is_empty(),
        }
    }
}
