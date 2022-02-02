// v1 format is anything older than 0.12.0
pub mod v1 {
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    use cosmwasm_std::Addr;
    use cw_storage_plus::Item;

    #[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
    pub struct Config {
        pub default_timeout: u64,
        pub gov_contract: Addr,
    }

    pub const CONFIG: Item<Config> = Item::new("ics20_config");
}
