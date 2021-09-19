use crate::module::FailingModule;
use crate::Module;
use cosmwasm_std::{DistributionMsg, Empty, StakingMsg, StakingQuery};

pub trait Staking: Module<ExecT = StakingMsg, QueryT = StakingQuery> {}

pub type FailingStaking = FailingModule<StakingMsg, StakingQuery>;

impl Staking for FailingStaking {}

pub trait Distribution: Module<ExecT = DistributionMsg, QueryT = Empty> {}

pub type FailingDistribution = FailingModule<DistributionMsg, Empty>;

impl Distribution for FailingDistribution {}
