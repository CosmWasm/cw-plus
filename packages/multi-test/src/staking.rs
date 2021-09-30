use cosmwasm_std::{Decimal, DistributionMsg, Empty, StakingMsg, StakingQuery};
use schemars::JsonSchema;

use crate::module::FailingModule;
use crate::Module;

// We need to expand on this, but we will need this to properly test out staking
#[derive(Clone, std::fmt::Debug, PartialEq, JsonSchema)]
pub enum StakingSudo {
    Slash {
        validator: String,
        percentage: Decimal,
    },
}

pub trait Staking: Module<ExecT = StakingMsg, QueryT = StakingQuery, SudoT = StakingSudo> {}

pub type FailingStaking = FailingModule<StakingMsg, StakingQuery, StakingSudo>;

impl Staking for FailingStaking {}

pub trait Distribution: Module<ExecT = DistributionMsg, QueryT = Empty, SudoT = Empty> {}

pub type FailingDistribution = FailingModule<DistributionMsg, Empty, Empty>;

impl Distribution for FailingDistribution {}
