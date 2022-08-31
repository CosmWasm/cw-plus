use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct InstantiateMsg {
    /// The minter is the only one who can create new tokens.
    /// This is designed for a base token platform that is controlled by an external program or
    /// contract.
    pub minter: String,
}
