use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct VersionResponse {
    pub version_require: String,
    pub supported_version: String,
    pub result: bool,
}
