use std::collections::{BTreeSet, VecDeque};

use anyhow::{anyhow, bail, Result as AnyResult};
use schemars::JsonSchema;

use cosmwasm_std::{
    coin, coins, ensure, ensure_eq, from_slice, to_binary, Addr, AllDelegationsResponse,
    AllValidatorsResponse, Api, BankMsg, Binary, BlockInfo, BondedDenomResponse, Coin, CustomQuery,
    Decimal, Delegation, DelegationResponse, DistributionMsg, Empty, Event, FullDelegation,
    Querier, StakingMsg, StakingQuery, Storage, Uint128, Validator, ValidatorResponse,
};
use cosmwasm_storage::{prefixed, prefixed_read};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

use crate::app::CosmosRouter;
use crate::executor::AppResponse;
use crate::{BankSudo, Module};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct StakeInfo {
    /// The block height when this stake was last edited. This is needed for slashing
    height: u64,

    /// The number of shares of this validator the staker has
    shares: Decimal,
}

impl StakeInfo {
    /// The stake of this delegator. Make sure to pass the correct validator in
    pub fn stake(&self, validator: &ValidatorInfo) -> Uint128 {
        self.shares / validator.total_shares * validator.stake
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct ValidatorInfo {
    /// The stakers that have staked with this validator
    stakers: BTreeSet<Addr>,
    /// The whole stake of all stakers
    stake: Uint128,
    /// The block height when this validator was last edited. This is needed for rewards calculation
    height: u64,
    /// The total number of shares this validator has issued, only used internally for calculating rewards
    total_shares: Decimal,
    /// The number of available rewards. This is updated whenever a
    available_rewards: Decimal,
}

impl ValidatorInfo {
    pub fn new(block_height: u64) -> Self {
        Self {
            stakers: BTreeSet::new(),
            stake: Uint128::zero(),
            height: block_height,
            total_shares: Decimal::zero(),
            available_rewards: Decimal::zero(),
        }
    }
    /// Returns the amount of shares a delegator gets for staking the given amount of tokens (bonded_denom) at this point in time.
    /// This should usually be `1:1` unless the delegator was slashed.
    pub fn shares_for(&self, stake: Uint128) -> Decimal {
        if self.stake.is_zero() {
            // first staker always gets 1:1
            Decimal::one()
        } else {
            Decimal::from_ratio(stake, 1u128) * self.total_shares
                / Decimal::from_ratio(self.stake, 1u128)
        }
    }
}

const STAKES: Map<(&Addr, &Addr), StakeInfo> = Map::new("stakes");
const VALIDATOR_MAP: Map<&Addr, Validator> = Map::new("validator_map");
/// Additional vec of validators, in case the `iterator` feature is disabled
const VALIDATORS: Item<Vec<Validator>> = Item::new("validators");
/// Contains additional info for each validator
const VALIDATOR_INFO: Map<&Addr, ValidatorInfo> = Map::new("validator_info");
// TODO: replace with `Deque`
// const UNBONDING_QUEUE: Item<VecDeque<(Addr, StakeInfo)>> = Item::new("unbonding_queue");

pub const NAMESPACE_STAKING: &[u8] = b"staking";

// We need to expand on this, but we will need this to properly test out staking
#[derive(Clone, std::fmt::Debug, PartialEq, Eq, JsonSchema)]
pub enum StakingSudo {
    Slash {
        validator: String,
        percentage: Decimal,
    },
    /// Causes the unbonding queue to be processed.
    /// This needs to be triggered manually, since there is no good place to do this right now
    ProcessQueue {},
}

pub trait Staking: Module<ExecT = StakingMsg, QueryT = StakingQuery, SudoT = StakingSudo> {}

pub trait Distribution: Module<ExecT = DistributionMsg, QueryT = Empty, SudoT = Empty> {}

pub struct StakeKeeper {
    module_addr: Addr,
    bonded_denom: String,
    /// time between unbonding and receiving tokens in seconds
    unbonding_time: u64,
}

impl Default for StakeKeeper {
    fn default() -> Self {
        Self::new()
    }
}

impl StakeKeeper {
    pub fn new() -> Self {
        StakeKeeper {
            // define this better?? it is an account for everything held by the staking keeper
            module_addr: Addr::unchecked("staking_module"),
            bonded_denom: "TOKEN".to_string(),
            unbonding_time: 60,
        }
    }

    pub fn init_stake(
        &self,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        account: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);

        self.add_stake(&mut storage, block, account, validator, amount)
    }

    /// Add a new validator available for staking
    pub fn add_validator(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        validator: Validator,
    ) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);
        let val_addr = api.addr_validate(&validator.address)?;
        if VALIDATOR_MAP.may_load(&storage, &val_addr)?.is_some() {
            bail!(
                "Cannot add validator {}, since a validator with that address already exists",
                val_addr
            );
        }

        VALIDATOR_MAP.save(&mut storage, &val_addr, &validator)?;
        let mut vec = VALIDATORS.may_load(&storage)?.unwrap_or_default();
        vec.push(validator);
        VALIDATORS.save(&mut storage, &vec)?;
        VALIDATOR_INFO.save(&mut storage, &val_addr, &ValidatorInfo::new(block.height))?;
        Ok(())
    }

    /// Calculates the staking reward for the given stake based on the fixed ratio set using
    /// `set_reward_ratio`
    fn calculate_rewards(&self, storage: &dyn Storage, stake: Uint128) -> AnyResult<Coin> {
        todo!("calculate rewards");
    }

    /// Returns the single validator with the given address (or `None` if there is no such validator)
    fn get_validator(&self, storage: &dyn Storage, address: &Addr) -> AnyResult<Option<Validator>> {
        Ok(VALIDATOR_MAP.may_load(storage, address)?)
    }

    /// Returns all available validators
    fn get_validators(&self, storage: &dyn Storage) -> AnyResult<Vec<Validator>> {
        Ok(VALIDATORS.may_load(storage)?.unwrap_or_default())
    }

    fn get_stake(
        &self,
        storage: &dyn Storage,
        account: &Addr,
        validator: &Addr,
    ) -> AnyResult<Uint128> {
        let val = STAKES.may_load(storage, (account, validator))?;
        let validator_info = VALIDATOR_INFO.may_load(storage, validator)?;
        Ok(val
            .zip(validator_info)
            .map(|(s, validator_info)| s.stake(&validator_info))
            .unwrap_or_default())
    }

    fn add_stake(
        &self,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        to_address: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        self.validate_denom(&amount)?;
        self.validate_nonzero(&amount)?;
        self.update_stake(storage, block, to_address, validator, amount.amount, false)
    }

    fn remove_stake(
        &self,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        from_address: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        self.validate_denom(&amount)?;
        self.validate_nonzero(&amount)?;
        self.update_stake(storage, block, from_address, validator, amount.amount, true)
    }

    fn update_stake(
        &self,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        delegator: &Addr,
        validator: &Addr,
        amount: impl Into<Uint128>,
        sub: bool,
    ) -> AnyResult<()> {
        let amount = amount.into();

        let mut validator_info = VALIDATOR_INFO
            .may_load(storage, validator)?
            .unwrap_or_else(|| ValidatorInfo::new(block.height));
        let mut stake_info = STAKES
            .may_load(storage, (delegator, validator))?
            .unwrap_or_else(|| StakeInfo {
                height: block.height,
                shares: Decimal::zero(),
            });

        // TODO: update rewards and validator_info.height

        if sub {
            // remove the corresponding amount of shares
            let shares = validator_info.shares_for(amount);
            stake_info.shares -= shares;

            validator_info.stake = validator_info.stake.checked_sub(amount)?;
            validator_info.total_shares -= shares;
        } else {
            let new_shares = validator_info.shares_for(amount);
            stake_info.shares += new_shares;

            validator_info.stake = validator_info.stake.checked_add(amount)?;
            validator_info.total_shares += new_shares;
        }

        // save updated values
        if stake_info.shares.is_zero() {
            // no more stake, so remove
            STAKES.remove(storage, (delegator, validator));
            validator_info.stakers.remove(delegator);
        } else {
            STAKES.save(storage, (delegator, validator), &stake_info)?;
            validator_info.stakers.insert(delegator.clone());
        }
        // save updated validator info
        VALIDATOR_INFO.save(storage, validator, &validator_info)?;

        Ok(())
    }

    fn slash(
        &self,
        storage: &mut dyn Storage,
        validator: &Addr,
        percentage: Decimal,
    ) -> AnyResult<()> {
        let mut validator_info = VALIDATOR_INFO
            .may_load(storage, validator)?
            .ok_or_else(|| anyhow!("validator not found"))?;

        // TODO: handle rewards? Either update them before slashing or set them to zero, depending on the slashing logic

        let remaining_percentage = Decimal::one() - percentage;
        validator_info.stake = validator_info.stake * remaining_percentage;

        // if the stake is completely gone, we clear all stakers and reinitialize the validator
        if validator_info.stake.is_zero() {
            // need to remove all stakes
            for delegator in validator_info.stakers.iter() {
                STAKES.remove(storage, (delegator, validator));
            }
            validator_info.stakers.clear();
            validator_info.total_shares = Decimal::zero();
        }
        VALIDATOR_INFO.save(storage, validator, &validator_info)?;
        Ok(())
    }

    fn process_queue(&self, storage: &mut dyn Storage) -> AnyResult<()> {
        // let mut queue = UNBONDING_QUEUE.may_load(storage)?.unwrap_or_default();

        // while queue.front().is_some() {
        //     let Some((delegator, info)) = queue.pop_front();
        // }
        // Ok(())

        todo!("process queue")
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

    fn validate_nonzero(&self, amount: &Coin) -> AnyResult<()> {
        ensure!(!amount.amount.is_zero(), anyhow!("cannot delegate 0 coins"));
        Ok(())
    }

    // Asserts that the given coin has the proper denominator
    fn validate_denom(&self, amount: &Coin) -> AnyResult<()> {
        ensure_eq!(
            amount.denom,
            self.bonded_denom,
            anyhow!(
                "cannot delegate coins of denominator {}, only of {}",
                amount.denom,
                self.bonded_denom
            )
        );
        Ok(())
    }

    // Asserts that the given coin has the proper denominator
    fn validate_percentage(&self, percentage: Decimal) -> AnyResult<()> {
        ensure!(percentage <= Decimal::one(), anyhow!("expected percentage"));
        Ok(())
    }
}

impl Staking for StakeKeeper {}

impl Module for StakeKeeper {
    type ExecT = StakingMsg;
    type QueryT = StakingQuery;
    type SudoT = StakingSudo;

    fn execute<ExecC, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: StakingMsg,
    ) -> AnyResult<AppResponse> {
        let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
        match msg {
            StakingMsg::Delegate { validator, amount } => {
                let validator = api.addr_validate(&validator)?;

                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L251-L256
                let events = vec![Event::new("delegate")
                    .add_attribute("validator", &validator)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))
                    .add_attribute("new_shares", amount.amount.to_string())]; // TODO: calculate shares?
                self.add_stake(
                    &mut staking_storage,
                    block,
                    &sender,
                    &validator,
                    amount.clone(),
                )?;
                // move money from sender account to this module (note we can controller sender here)
                router.execute(
                    api,
                    storage,
                    block,
                    sender,
                    BankMsg::Send {
                        to_address: self.module_addr.to_string(),
                        amount: vec![amount],
                    }
                    .into(),
                )?;
                Ok(AppResponse { events, data: None })
            }
            StakingMsg::Undelegate { validator, amount } => {
                let validator = api.addr_validate(&validator)?;

                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L378-L383
                let events = vec![Event::new("unbond")
                    .add_attribute("validator", &validator)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))
                    .add_attribute("completion_time", "2022-09-27T14:00:00+00:00")]; // TODO: actual date?
                self.remove_stake(
                    &mut staking_storage,
                    block,
                    &sender,
                    &validator,
                    amount.clone(),
                )?;
                // move token from this module to sender account
                // TODO: actually store this so it is released later after unbonding period
                // but showing how to do the payback
                router.execute(
                    api,
                    storage,
                    block,
                    self.module_addr.clone(),
                    BankMsg::Send {
                        to_address: sender.into_string(),
                        amount: vec![amount],
                    }
                    .into(),
                )?;

                // NB: when you need more tokens for staking rewards you can do something like:
                router.sudo(
                    api,
                    storage,
                    block,
                    BankSudo::Mint {
                        to_address: self.module_addr.to_string(),
                        amount: coins(123456000, "ucosm"),
                    }
                    .into(),
                )?;
                Ok(AppResponse { events, data: None })
            }
            StakingMsg::Redelegate {
                src_validator,
                dst_validator,
                amount,
            } => {
                let src_validator = api.addr_validate(&src_validator)?;
                let dst_validator = api.addr_validate(&dst_validator)?;
                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L316-L322
                let events = vec![Event::new("redelegate")
                    .add_attribute("source_validator", &src_validator)
                    .add_attribute("destination_validator", &dst_validator)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))];

                self.remove_stake(
                    &mut staking_storage,
                    block,
                    &sender,
                    &src_validator,
                    amount.clone(),
                )?;
                self.add_stake(&mut staking_storage, block, &sender, &dst_validator, amount)?;

                Ok(AppResponse { events, data: None })
            }
            m => bail!("Unsupported staking message: {:?}", m),
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        msg: StakingSudo,
    ) -> AnyResult<AppResponse> {
        let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
        match msg {
            StakingSudo::Slash {
                validator,
                percentage,
            } => {
                let validator = api.addr_validate(&validator)?;
                self.validate_percentage(percentage)?;

                self.slash(&mut staking_storage, &validator, percentage)?;

                Ok(AppResponse::default())
            }
            StakingSudo::ProcessQueue {} => {
                self.process_queue(&mut staking_storage)?;
                Ok(AppResponse::default())
            }
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
            StakingQuery::BondedDenom {} => Ok(to_binary(&BondedDenomResponse {
                denom: self.bonded_denom.clone(),
            })?),
            StakingQuery::AllDelegations { delegator } => {
                let delegator = api.addr_validate(&delegator)?;
                let validators = self.get_validators(&staking_storage)?;

                let res: AnyResult<Vec<Delegation>> = validators
                    .into_iter()
                    .map(|validator| {
                        let delegator = delegator.clone();
                        let amount = self.get_stake(
                            &staking_storage,
                            &delegator,
                            &Addr::unchecked(&validator.address),
                        )?;

                        Ok(Delegation {
                            delegator,
                            validator: validator.address,
                            amount: coin(amount.u128(), &self.bonded_denom),
                        })
                    })
                    .collect();

                Ok(to_binary(&AllDelegationsResponse { delegations: res? })?)
            }
            StakingQuery::Delegation {
                delegator,
                validator,
            } => {
                let validator_addr = Addr::unchecked(&validator);
                let validator_obj = self.get_validator(storage, &validator_addr)?;
                if validator_obj.is_none() {
                    bail!("non-existent validator {}", validator);
                }
                let delegator = api.addr_validate(&delegator)?;
                let stakes = match self.get_stake(&staking_storage, &delegator, &validator_addr) {
                    Ok(stakes) => stakes,
                    Err(_) => {
                        let response = DelegationResponse { delegation: None };
                        return Ok(to_binary(&response)?);
                    }
                };
                // calculate rewards using fixed ratio
                let reward = self.calculate_rewards(&staking_storage, stakes)?;
                let full_delegation_response = DelegationResponse {
                    delegation: Some(FullDelegation {
                        delegator,
                        validator,
                        amount: coin(stakes.u128(), &self.bonded_denom),
                        can_redelegate: coin(0, "testcoin"),
                        accumulated_rewards: vec![reward],
                    }),
                };

                let res = to_binary(&full_delegation_response)?;
                Ok(res)
            }
            StakingQuery::AllValidators {} => Ok(to_binary(&AllValidatorsResponse {
                validators: self.get_validators(&staking_storage)?,
            })?),
            StakingQuery::Validator { address } => Ok(to_binary(&ValidatorResponse {
                validator: self.get_validator(&staking_storage, &Addr::unchecked(address))?,
            })?),
            q => bail!("Unsupported staking sudo message: {:?}", q),
        }
    }
}

#[derive(Default)]
pub struct DistributionKeeper {}

impl DistributionKeeper {
    pub fn new() -> Self {
        DistributionKeeper {}
    }
}

impl Distribution for DistributionKeeper {}

impl Module for DistributionKeeper {
    type ExecT = DistributionMsg;
    type QueryT = Empty;
    type SudoT = Empty;

    fn execute<ExecC, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        sender: Addr,
        msg: DistributionMsg,
    ) -> AnyResult<AppResponse> {
        // let staking_storage = prefixed(storage, NAMESPACE_STAKING);
        match msg {
            // For now it ignores validator as I want to support only one
            DistributionMsg::WithdrawDelegatorReward { validator } => {
                let response: DelegationResponse = from_slice(&router.query(
                    api,
                    storage,
                    block,
                    cosmwasm_std::QueryRequest::Staking(StakingQuery::Delegation {
                        delegator: sender.to_string(),
                        validator: validator.clone(),
                    }),
                )?)?;
                let reward = &response.delegation.unwrap().accumulated_rewards[0];

                let events = vec![Event::new("withdraw_delegator_reward")
                    .add_attribute("validator", &validator)
                    .add_attribute("sender", &sender)
                    .add_attribute("amount", format!("{}{}", reward.amount, reward.denom))];
                // TODO: add balance to sender by sending BankMsg transfer
                Ok(AppResponse { events, data: None })
            }
            m => bail!("Unsupported distribution message: {:?}", m),
        }
    }

    fn sudo<ExecC, QueryC>(
        &self,
        _api: &dyn Api,
        _storage: &mut dyn Storage,
        _router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        _block: &BlockInfo,
        _msg: Empty,
    ) -> AnyResult<AppResponse> {
        bail!("Something went wrong - Distribution doesn't have sudo messages")
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _querier: &dyn Querier,
        _block: &BlockInfo,
        _request: Empty,
    ) -> AnyResult<Binary> {
        bail!("Something went wrong - Distribution doesn't have query messages")
    }
}

#[cfg(test)]
mod test {
    use crate::app::MockRouter;

    use super::*;

    use cosmwasm_std::testing::{mock_env, MockApi, MockStorage};

    #[test]
    fn add_get_validators() {
        let api = MockApi::default();
        let mut store = MockStorage::new();
        let stake = StakeKeeper::new();
        let block = mock_env().block;

        // add validator
        let valoper1 = Validator {
            address: "testvaloper1".to_string(),
            commission: Decimal::percent(10),
            max_commission: Decimal::percent(20),
            max_change_rate: Decimal::percent(1),
        };
        stake
            .add_validator(&api, &mut store, &block, valoper1.clone())
            .unwrap();

        // get it
        let staking_storage = prefixed_read(&store, NAMESPACE_STAKING);
        let val = stake
            .get_validator(
                &staking_storage,
                &api.addr_validate("testvaloper1").unwrap(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(val, valoper1);

        // try to add with same address
        let valoper1_fake = Validator {
            address: "testvaloper1".to_string(),
            commission: Decimal::percent(1),
            max_commission: Decimal::percent(10),
            max_change_rate: Decimal::percent(100),
        };
        stake
            .add_validator(&api, &mut store, &block, valoper1_fake)
            .unwrap_err();

        // should still be original value
        let staking_storage = prefixed_read(&store, NAMESPACE_STAKING);
        let val = stake
            .get_validator(
                &staking_storage,
                &api.addr_validate("testvaloper1").unwrap(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(val, valoper1);
    }

    #[test]
    fn validator_slashing() {
        let api = MockApi::default();
        let router = MockRouter::default();
        let mut store = MockStorage::new();
        let stake = StakeKeeper::new();
        let block = mock_env().block;

        let delegator = Addr::unchecked("delegator");
        let validator = api.addr_validate("testvaloper1").unwrap();

        // add validator
        let valoper1 = Validator {
            address: "testvaloper1".to_string(),
            commission: Decimal::percent(10),
            max_commission: Decimal::percent(20),
            max_change_rate: Decimal::percent(1),
        };
        stake
            .add_validator(&api, &mut store, &block, valoper1)
            .unwrap();

        // stake 100 tokens
        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        stake
            .add_stake(
                &mut staking_storage,
                &block,
                &delegator,
                &validator,
                coin(100, "TOKEN"),
            )
            .unwrap();

        // slash 50%
        stake
            .sudo(
                &api,
                &mut store,
                &router,
                &block,
                StakingSudo::Slash {
                    validator: "testvaloper1".to_string(),
                    percentage: Decimal::percent(50),
                },
            )
            .unwrap();

        // check stake
        let staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        let stake_left = stake
            .get_stake(&staking_storage, &delegator, &validator)
            .unwrap();
        assert_eq!(stake_left.u128(), 50, "should have slashed 50%");

        // slash all
        stake
            .sudo(
                &api,
                &mut store,
                &router,
                &block,
                StakingSudo::Slash {
                    validator: "testvaloper1".to_string(),
                    percentage: Decimal::percent(100),
                },
            )
            .unwrap();

        // check stake
        let staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        let stake_left = stake
            .get_stake(&staking_storage, &delegator, &validator)
            .unwrap();
        assert_eq!(stake_left.u128(), 0, "should have slashed whole stake");
    }
}
