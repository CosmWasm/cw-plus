// use cosmwasm_schema::cw_serde;
// use cosmwasm_std::{Uint128};

// #[cw_serde]
#[derive(Debug, PartialEq)]
pub struct VersionResponse {
    pub version_require: String,
    pub supported_version: String,
    pub result: bool,
}
