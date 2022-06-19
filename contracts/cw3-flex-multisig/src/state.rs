use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, QuerierWrapper};
use cw4::Cw4Contract;
use cw_storage_plus::Item;
use cw_utils::{Duration, Threshold};

use crate::error::ContractError;

/// Defines who is able to execute proposals once passed
#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub enum Executor {
    /// Any member of the voting group, even with 0 points
    Member,
    /// Only the given address
    Only(Addr),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct Config {
    pub threshold: Threshold,
    pub max_voting_period: Duration,
    // Total weight and voters are queried from this contract
    pub group_addr: Cw4Contract,
    // who is able to execute passed proposals
    // None means that anyone can execute
    pub executor: Option<Executor>,
}

impl Config {
    // Executor can be set in 3 ways:
    // - Member: any member of the voting group is authorized
    // - Only: only passed address is authorized
    // - None: Everyone are authorized
    pub fn authorize(&self, querier: &QuerierWrapper, sender: &Addr) -> Result<(), ContractError> {
        if let Some(executor) = &self.executor {
            match executor {
                Executor::Member => {
                    self.group_addr
                        .is_member(querier, sender, None)?
                        .ok_or(ContractError::Unauthorized {})?;
                }
                Executor::Only(addr) => {
                    if addr != sender {
                        return Err(ContractError::Unauthorized {});
                    }
                }
            }
        }
        Ok(())
    }
}

// unique items
pub const CONFIG: Item<Config> = Item::new("config");
