use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    /// The minter is the only one who can create new tokens.
    /// This is designed for a base token platform that is controlled by an external program or
    /// contract.
    pub minter: String,
}
