use anyhow::{bail, Result as AnyResult};
use schemars::JsonSchema;

use cosmwasm_std::{
    Addr, Api, Binary, BlockInfo, Coin, Decimal, DistributionMsg, Empty, Event, Querier,
    StakingMsg, StakingQuery, Storage, DelegationResponse, FullDelegation, to_binary, coin, Uint128
};
use cosmwasm_storage::{prefixed, prefixed_read};
use cw_storage_plus::Map;
use cw_utils::NativeBalance;

use crate::app::CosmosRouter;
use crate::executor::AppResponse;
use crate::module::FailingModule;
use crate::Module;

const STAKES: Map<&Addr, NativeBalance> = Map::new("stakes");

pub const NAMESPACE_STAKING: &[u8] = b"staking";

// We need to expand on this, but we will need this to properly test out staking
#[derive(Clone, std::fmt::Debug, PartialEq, Eq, JsonSchema)]
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

#[derive(Default)]
pub struct StakeKeeper {}

impl StakeKeeper {
    pub fn new() -> Self {
        StakeKeeper {}
    }

    pub fn init_balance(
        &self,
        storage: &mut dyn Storage,
        account: &Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);
        self.set_balance(&mut storage, account, amount)
    }

    fn get_stakes(&self, storage: &dyn Storage, account: &Addr) -> AnyResult<Vec<Coin>> {
        let val = STAKES.may_load(storage, account)?;
        Ok(val.unwrap_or_default().into_vec())
    }

    fn set_balance(
        &self,
        storage: &mut dyn Storage,
        account: &Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let mut stake = NativeBalance(amount);
        stake.normalize();
        STAKES.save(storage, account, &stake).map_err(Into::into)
    }

    fn add_stake(
        &self,
        storage: &mut dyn Storage,
        to_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let amount = self.normalize_amount(amount)?;
        let b = self.get_stakes(storage, &to_address)?;
        let b = NativeBalance(b) + NativeBalance(amount);
        self.set_balance(storage, &to_address, b.into_vec())
    }

    fn remove_stake(
        &self,
        storage: &mut dyn Storage,
        from_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let amount = self.normalize_amount(amount)?;
        let a = self.get_stakes(storage, &from_address)?;
        let a = (NativeBalance(a) - amount)?;
        self.set_balance(storage, &from_address, a.into_vec())
    }

    /// Filters out all 0 value coins and returns an error if the resulting Vec is empty
    fn normalize_amount(&self, amount: Vec<Coin>) -> AnyResult<Vec<Coin>> {
        let res: Vec<_> = amount.into_iter().filter(|x| !x.amount.is_zero()).collect();
        if res.is_empty() {
            bail!("Cannot transfer empty coins amount")
        } else {
            Ok(res)
        }
    }
}

impl Staking for StakeKeeper {}

impl Module for StakeKeeper {
    type ExecT = StakingMsg;
    type QueryT = StakingQuery;
    type SudoT = StakingSudo;

    fn execute<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        sender: Addr,
        msg: StakingMsg,
    ) -> AnyResult<AppResponse> {
        let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
        match msg {
            StakingMsg::Delegate { validator, amount } => {
                let events = vec![Event::new("delegate")
                    .add_attribute("recipient", &validator)
                    .add_attribute("sender", &sender)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))];
                self.add_stake(&mut staking_storage, sender, vec![amount])?;
                Ok(AppResponse { events, data: None })
            },
            StakingMsg::Undelegate { validator, amount } => {
                let events = vec![Event::new("undelegate")
                    .add_attribute("from", &validator)
                    .add_attribute("to", &sender)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))];
                self.remove_stake(&mut staking_storage, sender, vec![amount])?;
                Ok(AppResponse { events, data: None })
            }
            m => bail!("Unsupported staking message: {:?}", m),
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        msg: StakingSudo,
    ) -> AnyResult<AppResponse> {
        match msg {
            s => bail!("Unsupported staking sudo message: {:?}", s),
        }
    }

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        request: StakingQuery,
    ) -> AnyResult<Binary> {
        let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);
        match request {
            StakingQuery::Delegation { delegator, validator } => {
                // for now validator is ignored, as I want to support only one validator
                let delegator = api.addr_validate(&delegator)?;
                let stakes = match self.get_stakes(&staking_storage, &delegator) {
                    Ok(stakes) => stakes[0].clone(),
                    Err(_) => {
                        let response = DelegationResponse { delegation: None };
                        return Ok(to_binary(&response)?);
                    }
                };
                // set fixed reward ratio 1:10 per delegated amoutn
                let reward = coin((stakes.amount / Uint128::new(10)).u128(), stakes.denom.clone());
                let full_delegation_response = FullDelegation {
                    delegator,
                    validator,
                    amount: stakes,
                    can_redelegate: coin(0, "testcoin"),
                    accumulated_rewards: vec![reward],
                };
                Ok(to_binary(&full_delegation_response)?)
            }
            q => bail!("Unsupported staking sudo message: {:?}", q),
        }
    }
}
