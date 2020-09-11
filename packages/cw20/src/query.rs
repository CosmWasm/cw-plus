use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Uint128};

use cw0::Expiration;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw20QueryMsg {
    /// Returns the current balance of the given address, 0 if unset.
    /// Return type: BalanceResponse.
    Balance { address: HumanAddr },
    /// Returns metadata on the contract - name, decimals, supply, etc.
    /// Return type: TokenInfoResponse.
    TokenInfo {},
    /// Only with "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    /// Return type: AllowanceResponse.
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
    },
    /// Only with "mintable" extension.
    /// Returns who can mint and how much.
    /// Return type: MinterResponse.
    Minter {},
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this owner has approved. Supports pagination.
    /// Return type: AllAllowancesResponse.
    AllAllowances {
        owner: HumanAddr,
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
    /// Only with "enumerable" extension
    /// Returns all accounts that have balances. Supports pagination.
    /// Return type: AllAccountsResponse.
    AllAccounts {
        start_after: Option<HumanAddr>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BalanceResponse {
    pub balance: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct TokenInfoResponse {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AllowanceResponse {
    pub allowance: Uint128,
    pub expires: Expiration,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct MinterResponse {
    pub minter: HumanAddr,
    /// cap is how many more tokens can be issued by the minter
    pub cap: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AllowanceInfo {
    #[serde(flatten)]
    pub allowance_response: AllowanceResponse,
    pub spender: HumanAddr,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AllAllowancesResponse {
    pub allowances: Vec<AllowanceInfo>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AllAccountsResponse {
    pub accounts: Vec<HumanAddr>,
}

#[cfg(test)]
mod test {
    use super::*;
    use cosmwasm_schema::schema_for;
    use serde_json;
    use valico::json_schema;

    #[test]
    fn it_should_ser_de() {
        let response = AllowanceResponse {
            allowance: Uint128(500000),
            expires: Expiration::AtTime(1234),
        };

        let info = AllowanceInfo {
            allowance_response: response,
            spender: HumanAddr::from("spender"),
        };
        let allowance_info = serde_json::to_value(info.clone()).unwrap();
        let serialized = serde_json::to_string(&allowance_info).unwrap();
        let deserialized: AllowanceInfo = serde_json::from_str(&serialized).unwrap();
        assert!(info.eq(&deserialized));
    }

    #[test]
    fn it_should_make_correct_schema() {
        let response = AllowanceResponse {
            allowance: Uint128(500000),
            expires: Expiration::AtTime(1234),
        };

        let info = AllowanceInfo {
            allowance_response: response.clone(),
            spender: HumanAddr::from("spender"),
        };

        let allowance_info = serde_json::to_value(info).unwrap();
        let allowance_res = serde_json::to_value(response).unwrap();

        let schema = schema_for!(AllowanceInfo);
        let schema_string = serde_json::to_string(&schema).unwrap();
        let schema_json: serde_json::Value = serde_json::from_str(&schema_string).unwrap();

        let mut scope = json_schema::Scope::new();
        let r_schema = scope
            .compile_and_return(schema_json.clone(), true)
            .ok()
            .unwrap();

        println!("AllowanceInfo json schema \n {:#?}", schema_json);

        assert_eq!(r_schema.validate(&allowance_info).is_valid(), true);

        assert_eq!(r_schema.validate(&allowance_res).is_valid(), false);
    }
}
