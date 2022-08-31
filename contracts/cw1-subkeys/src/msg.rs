use schemars::JsonSchema;

use std::fmt;

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Coin, CosmosMsg, Empty};
use cw_utils::{Expiration, NativeBalance};

use crate::state::Permissions;

#[cw_serde]
pub enum ExecuteMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Execute requests the contract to re-dispatch all these messages with the
    /// contract's address as sender. Every implementation has it's own logic to
    /// determine in
    Execute { msgs: Vec<CosmosMsg<T>> },
    /// Freeze will make a mutable contract immutable, must be called by an admin
    Freeze {},
    /// UpdateAdmins will change the admin set of the contract, must be called by an existing admin,
    /// and only works if the contract is mutable
    UpdateAdmins { admins: Vec<String> },

    /// Add an allowance to a given subkey (subkey must not be admin)
    IncreaseAllowance {
        spender: String,
        amount: Coin,
        expires: Option<Expiration>,
    },
    /// Decreases an allowance for a given subkey (subkey must not be admin)
    DecreaseAllowance {
        spender: String,
        amount: Coin,
        expires: Option<Expiration>,
    },

    // Setups up permissions for a given subkey.
    SetPermissions {
        spender: String,
        permissions: Permissions,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg<T = Empty>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    /// Shows all admins and whether or not it is mutable
    #[returns(cw1_whitelist::msg::AdminListResponse)]
    AdminList {},
    /// Get the current allowance for the given subkey (how much it can spend)
    #[returns(crate::state::Allowance)]
    Allowance { spender: String },
    /// Get the current permissions for the given subkey (how much it can spend)
    #[returns(PermissionsInfo)]
    Permissions { spender: String },
    /// Checks permissions of the caller on this proxy.
    /// If CanExecute returns true then a call to `Execute` with the same message,
    /// before any further state changes, should also succeed.
    #[returns(cw1::CanExecuteResponse)]
    CanExecute { sender: String, msg: CosmosMsg<T> },
    /// Gets all Allowances for this contract
    #[returns(AllAllowancesResponse)]
    AllAllowances {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Gets all Permissions for this contract
    #[returns(AllPermissionsResponse)]
    AllPermissions {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct AllAllowancesResponse {
    pub allowances: Vec<AllowanceInfo>,
}

#[cfg(test)]
impl AllAllowancesResponse {
    pub fn canonical(mut self) -> Self {
        self.allowances = self
            .allowances
            .into_iter()
            .map(AllowanceInfo::canonical)
            .collect();
        self.allowances.sort_by(AllowanceInfo::cmp_by_spender);
        self
    }
}

#[cw_serde]
pub struct AllowanceInfo {
    pub spender: String,
    pub balance: NativeBalance,
    pub expires: Expiration,
}

#[cfg(test)]
impl AllowanceInfo {
    /// Utility function providing some ordering to be used with `slice::sort_by`.
    ///
    /// Note, that this doesn't implement full ordering - items with same spender but differing on
    /// permissions, would be considered equal, however as spender is a unique key in any valid
    /// state this is enough for testing purposes.
    ///
    /// Example:
    ///
    /// ```
    /// # use cw_utils::{Expiration, NativeBalance};
    /// # use cw1_subkeys::msg::AllowanceInfo;
    /// # use cosmwasm_schema::{cw_serde, QueryResponses};use cosmwasm_std::coin;
    ///
    /// let mut allows = vec![AllowanceInfo {
    ///   spender: "spender2".to_owned(),
    ///   balance: NativeBalance(vec![coin(1, "token1")]),
    ///   expires: Expiration::Never {},
    /// }, AllowanceInfo {
    ///   spender: "spender1".to_owned(),
    ///   balance: NativeBalance(vec![coin(2, "token2")]),
    ///   expires: Expiration::Never {},
    /// }];
    ///
    /// allows.sort_by(AllowanceInfo::cmp_by_spender);
    ///
    /// assert_eq!(
    ///   allows.into_iter().map(|allow| allow.spender).collect::<Vec<_>>(),
    ///   vec!["spender1".to_owned(), "spender2".to_owned()]
    /// );
    /// ```
    pub fn cmp_by_spender(left: &Self, right: &Self) -> std::cmp::Ordering {
        left.spender.cmp(&right.spender)
    }

    pub fn canonical(mut self) -> Self {
        self.balance.normalize();
        self
    }
}

#[cw_serde]
pub struct PermissionsInfo {
    pub spender: String,
    pub permissions: Permissions,
}

#[cfg(any(test, feature = "test-utils"))]
impl PermissionsInfo {
    /// Utility function providing some ordering to be used with `slice::sort_by`.
    ///
    /// Note, that this doesn't implement full ordering - items with same spender but differing on
    /// permissions, would be considered equal, however as spender is a unique key in any valid
    /// state this is enough for testing purposes.
    ///
    /// Example:
    ///
    /// ```
    /// # use cw1_subkeys::msg::PermissionsInfo;
    /// # use cw1_subkeys::state::Permissions;
    ///
    /// let mut perms = vec![PermissionsInfo {
    ///   spender: "spender2".to_owned(),
    ///   permissions: Permissions::default(),
    /// }, PermissionsInfo {
    ///   spender: "spender1".to_owned(),
    ///   permissions: Permissions::default(),
    /// }];
    ///
    /// perms.sort_by(PermissionsInfo::cmp_by_spender);
    ///
    /// assert_eq!(
    ///   perms.into_iter().map(|perm| perm.spender).collect::<Vec<_>>(),
    ///   vec!["spender1".to_owned(), "spender2".to_owned()]
    /// );
    /// ```
    pub fn cmp_by_spender(left: &Self, right: &Self) -> std::cmp::Ordering {
        left.spender.cmp(&right.spender)
    }
}

#[cw_serde]
pub struct AllPermissionsResponse {
    pub permissions: Vec<PermissionsInfo>,
}
