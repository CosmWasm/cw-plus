use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cw0::{Expiration, NativeBalance};
use cw_storage_plus::Map;

use std::fmt;

// Permissions struct defines users message execution permissions.
// Could have implemented permissions for each cosmos module(StakingPermissions, GovPermissions etc...)
// But that meant a lot of code for each module. Keeping the permissions inside one struct is more
// optimal. Define other modules permissions here.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default, Copy)]
pub struct Permissions {
    pub delegate: bool,
    pub redelegate: bool,
    pub undelegate: bool,
    pub withdraw: bool,
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "staking: {{ delegate: {}, redelegate: {}, undelegate: {}, withdraw: {} }}",
            self.delegate, self.redelegate, self.undelegate, self.withdraw
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema, Default)]
pub struct Allowance {
    pub balance: NativeBalance,
    pub expires: Expiration,
}

pub const PERMISSIONS: Map<&[u8], Permissions> = Map::new("permissions");
pub const ALLOWANCES: Map<&[u8], Allowance> = Map::new("permissions");
