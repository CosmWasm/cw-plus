use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Result as AnyResult};
use schemars::JsonSchema;

use cosmwasm_std::{
    coin, ensure, ensure_eq, to_binary, Addr, AllDelegationsResponse, AllValidatorsResponse, Api,
    BankMsg, Binary, BlockInfo, BondedDenomResponse, Coin, CustomQuery, Decimal, Delegation,
    DelegationResponse, DistributionMsg, Empty, Event, FullDelegation, Querier, StakingMsg,
    StakingQuery, Storage, Timestamp, Uint128, Validator, ValidatorResponse,
};
use cosmwasm_storage::{prefixed, prefixed_read};
use cw_storage_plus::{Deque, Item, Map};
use serde::{Deserialize, Serialize};

use crate::app::CosmosRouter;
use crate::executor::AppResponse;
use crate::{BankSudo, Module};

// Contains some general staking parameters
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StakingInfo {
    /// The denominator of the staking token
    bonded_denom: String,
    /// Time between unbonding and receiving tokens in seconds
    unbonding_time: u64,
    /// Interest rate per year (60 * 60 * 24 * 365 seconds)
    apr: Decimal,
}

impl Default for StakingInfo {
    fn default() -> Self {
        StakingInfo {
            bonded_denom: "TOKEN".to_string(),
            unbonding_time: 60,
            apr: Decimal::percent(10),
        }
    }
}

/// The number of stake and rewards of this validator the staker has. These can be fractional in case of slashing.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, JsonSchema)]
struct Shares {
    stake: Decimal,
    rewards: Decimal,
}

impl Shares {
    /// Calculates the share of validator rewards that should be given to this staker.
    pub fn share_of_rewards(&self, validator: &ValidatorInfo, rewards: Decimal) -> Decimal {
        rewards * self.stake / validator.stake
    }
}

/// Holds some operational data about a validator
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
struct ValidatorInfo {
    /// The stakers that have staked with this validator.
    /// We need to track them for updating their rewards.
    stakers: BTreeSet<Addr>,
    /// The whole stake of all stakers
    stake: Uint128,
    /// The block time when this validator's rewards were last update. This is needed for rewards calculation.
    last_rewards_calculation: Timestamp,
}

impl ValidatorInfo {
    pub fn new(block_time: Timestamp) -> Self {
        Self {
            stakers: BTreeSet::new(),
            stake: Uint128::zero(),
            last_rewards_calculation: block_time,
        }
    }
}

const STAKING_INFO: Item<StakingInfo> = Item::new("staking_info");
const STAKES: Map<(&Addr, &Addr), Shares> = Map::new("stakes");
const VALIDATOR_MAP: Map<&Addr, Validator> = Map::new("validator_map");
/// Additional vec of validators, in case the `iterator` feature is disabled
const VALIDATORS: Deque<Validator> = Deque::new("validators");
/// Contains additional info for each validator
const VALIDATOR_INFO: Map<&Addr, ValidatorInfo> = Map::new("validator_info");
/// The queue of unbonding operations. This is needed because unbonding has a waiting time. See [`StakeKeeper`]
const UNBONDING_QUEUE: Deque<(Addr, Timestamp, u128)> = Deque::new("unbonding_queue");

pub const NAMESPACE_STAKING: &[u8] = b"staking";

// We need to expand on this, but we will need this to properly test out staking
#[derive(Clone, std::fmt::Debug, PartialEq, Eq, JsonSchema)]
pub enum StakingSudo {
    /// Slashes the given percentage of the validator's stake.
    /// For now, you cannot slash retrospectively in tests.
    Slash {
        validator: String,
        percentage: Decimal,
    },
    /// Causes the unbonding queue to be processed.
    /// This needs to be triggered manually, since there is no good place to do this right now.
    /// In cosmos-sdk, this is done in `EndBlock`, but we don't have that here.
    ProcessQueue {},
}

pub trait Staking: Module<ExecT = StakingMsg, QueryT = StakingQuery, SudoT = StakingSudo> {}

pub trait Distribution: Module<ExecT = DistributionMsg, QueryT = Empty, SudoT = Empty> {}

pub struct StakeKeeper {
    module_addr: Addr,
}

impl Default for StakeKeeper {
    fn default() -> Self {
        Self::new()
    }
}

impl StakeKeeper {
    pub fn new() -> Self {
        StakeKeeper {
            // The address of the staking module. This holds all staked tokens.
            module_addr: Addr::unchecked("staking_module"),
        }
    }

    /// Provides some general parameters to the stake keeper
    pub fn setup(&self, storage: &mut dyn Storage, staking_info: StakingInfo) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);

        STAKING_INFO.save(&mut storage, &staking_info)?;
        Ok(())
    }

    pub fn init_stake(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        account: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        let mut storage = prefixed(storage, NAMESPACE_STAKING);

        self.add_stake(api, &mut storage, block, account, validator, amount)
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
        VALIDATORS.push_back(&mut storage, &validator)?;
        VALIDATOR_INFO.save(&mut storage, &val_addr, &ValidatorInfo::new(block.time))?;
        Ok(())
    }

    fn get_staking_info(staking_storage: &dyn Storage) -> AnyResult<StakingInfo> {
        Ok(STAKING_INFO.may_load(staking_storage)?.unwrap_or_default())
    }

    /// Returns the rewards of the given delegator at the given validator
    pub fn get_rewards(
        &self,
        storage: &dyn Storage,
        block: &BlockInfo,
        delegator: &Addr,
        validator: &Addr,
    ) -> AnyResult<Option<Coin>> {
        let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);

        let validator_obj = match self.get_validator(&staking_storage, validator)? {
            Some(validator) => validator,
            None => bail!("non-existent validator {}", validator),
        };
        // calculate rewards using fixed ratio
        let shares = match STAKES.load(&staking_storage, (delegator, validator)) {
            Ok(stakes) => stakes,
            Err(_) => {
                return Ok(None);
            }
        };
        let validator_info = VALIDATOR_INFO.load(&staking_storage, validator)?;

        Self::get_rewards_internal(
            &staking_storage,
            block,
            &shares,
            &validator_obj,
            &validator_info,
        )
        .map(Some)
    }

    fn get_rewards_internal(
        staking_storage: &dyn Storage,
        block: &BlockInfo,
        shares: &Shares,
        validator: &Validator,
        validator_info: &ValidatorInfo,
    ) -> AnyResult<Coin> {
        let staking_info = Self::get_staking_info(staking_storage)?;

        // calculate missing rewards without updating the validator to reduce rounding errors
        let new_validator_rewards = Self::calculate_rewards(
            block.time,
            validator_info.last_rewards_calculation,
            staking_info.apr,
            validator.commission,
            validator_info.stake,
        );

        // calculate the delegator's share of those
        let delegator_rewards =
            shares.rewards + shares.share_of_rewards(validator_info, new_validator_rewards);

        Ok(Coin {
            denom: staking_info.bonded_denom,
            amount: Uint128::new(1) * delegator_rewards, // multiplying by 1 to convert Decimal to Uint128
        })
    }

    /// Calculates the rewards that are due since the last calculation.
    fn calculate_rewards(
        current_time: Timestamp,
        since: Timestamp,
        interest_rate: Decimal,
        validator_commission: Decimal,
        stake: Uint128,
    ) -> Decimal {
        // calculate time since last update (in seconds)
        let time_diff = current_time.minus_seconds(since.seconds()).seconds();

        // using decimal here to reduce rounding error when calling this function a lot
        let reward = Decimal::from_ratio(stake, 1u128)
            * interest_rate
            * Decimal::from_ratio(time_diff, 1u128)
            / Decimal::from_ratio(60u128 * 60 * 24 * 365, 1u128);
        let commission = reward * validator_commission;

        reward - commission
    }

    /// Updates the staking reward for the given validator and their stakers
    /// It saves the validator info and it's stakers, so make sure not to overwrite that.
    /// Always call this to update rewards before changing anything that influences future rewards.
    fn update_rewards(
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        validator: &Addr,
    ) -> AnyResult<()> {
        let staking_info = Self::get_staking_info(staking_storage)?;

        let mut validator_info = VALIDATOR_INFO
            .may_load(staking_storage, validator)?
            .ok_or_else(|| anyhow!("validator not found"))?;

        let validator_obj = VALIDATOR_MAP.load(staking_storage, validator)?;

        if validator_info.last_rewards_calculation >= block.time {
            return Ok(());
        }

        let new_rewards = Self::calculate_rewards(
            block.time,
            validator_info.last_rewards_calculation,
            staking_info.apr,
            validator_obj.commission,
            validator_info.stake,
        );

        // update validator info and delegators
        if !new_rewards.is_zero() {
            validator_info.last_rewards_calculation = block.time;

            // save updated validator
            VALIDATOR_INFO.save(staking_storage, validator, &validator_info)?;

            let validator_addr = api.addr_validate(&validator_obj.address)?;
            // update all delegators
            for staker in validator_info.stakers.iter() {
                STAKES.update(
                    staking_storage,
                    (staker, &validator_addr),
                    |shares| -> AnyResult<_> {
                        let mut shares =
                            shares.expect("all stakers in validator_info should exist");
                        shares.rewards += shares.share_of_rewards(&validator_info, new_rewards);
                        Ok(shares)
                    },
                )?;
            }
        }
        Ok(())
    }

    /// Returns the single validator with the given address (or `None` if there is no such validator)
    fn get_validator(
        &self,
        staking_storage: &dyn Storage,
        address: &Addr,
    ) -> AnyResult<Option<Validator>> {
        Ok(VALIDATOR_MAP.may_load(staking_storage, address)?)
    }

    /// Returns all available validators
    fn get_validators(&self, staking_storage: &dyn Storage) -> AnyResult<Vec<Validator>> {
        let res: Result<_, _> = VALIDATORS.iter(staking_storage)?.collect();
        Ok(res?)
    }

    fn get_stake(
        &self,
        staking_storage: &dyn Storage,
        account: &Addr,
        validator: &Addr,
    ) -> AnyResult<Option<Coin>> {
        let shares = STAKES.may_load(staking_storage, (account, validator))?;
        let staking_info = Self::get_staking_info(staking_storage)?;

        Ok(shares.map(|shares| {
            Coin {
                denom: staking_info.bonded_denom,
                amount: Uint128::new(1) * shares.stake, // multiplying by 1 to convert Decimal to Uint128
            }
        }))
    }

    fn add_stake(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        to_address: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        self.validate_denom(staking_storage, &amount)?;
        self.validate_nonzero(&amount)?;
        self.update_stake(
            api,
            staking_storage,
            block,
            to_address,
            validator,
            amount.amount,
            false,
        )
    }

    fn remove_stake(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        from_address: &Addr,
        validator: &Addr,
        amount: Coin,
    ) -> AnyResult<()> {
        self.validate_denom(staking_storage, &amount)?;
        self.validate_nonzero(&amount)?;
        self.update_stake(
            api,
            staking_storage,
            block,
            from_address,
            validator,
            amount.amount,
            true,
        )
    }

    fn update_stake(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        delegator: &Addr,
        validator: &Addr,
        amount: impl Into<Uint128>,
        sub: bool,
    ) -> AnyResult<()> {
        let amount = amount.into();

        if amount.is_zero() {
            return Ok(());
        }

        // update rewards for this validator
        Self::update_rewards(api, staking_storage, block, validator)?;

        // now, we can update the stake of the delegator and validator
        let mut validator_info = VALIDATOR_INFO
            .may_load(staking_storage, validator)?
            .unwrap_or_else(|| ValidatorInfo::new(block.time));
        let mut shares = STAKES
            .may_load(staking_storage, (delegator, validator))?
            .unwrap_or_default();
        let amount_dec = Decimal::from_ratio(amount, 1u128);
        if sub {
            if amount_dec > shares.stake {
                bail!("insufficient stake");
            }
            shares.stake -= amount_dec;
            validator_info.stake = validator_info.stake.checked_sub(amount)?;
        } else {
            shares.stake += amount_dec;
            validator_info.stake = validator_info.stake.checked_add(amount)?;
        }

        // save updated values
        if shares.stake.is_zero() {
            // no more stake, so remove
            STAKES.remove(staking_storage, (delegator, validator));
            validator_info.stakers.remove(delegator);
        } else {
            STAKES.save(staking_storage, (delegator, validator), &shares)?;
            validator_info.stakers.insert(delegator.clone());
        }
        // save updated validator info
        VALIDATOR_INFO.save(staking_storage, validator, &validator_info)?;

        Ok(())
    }

    fn slash(
        &self,
        api: &dyn Api,
        staking_storage: &mut dyn Storage,
        block: &BlockInfo,
        validator: &Addr,
        percentage: Decimal,
    ) -> AnyResult<()> {
        // calculate rewards before slashing
        Self::update_rewards(api, staking_storage, block, validator)?;

        // update stake of validator and stakers
        let mut validator_info = VALIDATOR_INFO
            .may_load(staking_storage, validator)?
            .ok_or_else(|| anyhow!("validator not found"))?;

        let remaining_percentage = Decimal::one() - percentage;
        validator_info.stake = validator_info.stake * remaining_percentage;

        // if the stake is completely gone, we clear all stakers and reinitialize the validator
        if validator_info.stake.is_zero() {
            // need to remove all stakes
            for delegator in validator_info.stakers.iter() {
                STAKES.remove(staking_storage, (delegator, validator));
            }
            validator_info.stakers.clear();
        } else {
            // otherwise we update all stakers
            for delegator in validator_info.stakers.iter() {
                STAKES.update(
                    staking_storage,
                    (delegator, validator),
                    |stake| -> AnyResult<_> {
                        let mut stake = stake.expect("all stakers in validator_info should exist");
                        stake.stake *= remaining_percentage;

                        Ok(stake)
                    },
                )?;
            }
        }
        VALIDATOR_INFO.save(staking_storage, validator, &validator_info)?;
        Ok(())
    }

    fn validate_nonzero(&self, amount: &Coin) -> AnyResult<()> {
        ensure!(!amount.amount.is_zero(), anyhow!("cannot delegate 0 coins"));
        Ok(())
    }

    // Asserts that the given coin has the proper denominator
    fn validate_denom(&self, staking_storage: &dyn Storage, amount: &Coin) -> AnyResult<()> {
        let staking_info = Self::get_staking_info(staking_storage)?;
        ensure_eq!(
            amount.denom,
            staking_info.bonded_denom,
            anyhow!(
                "cannot delegate coins of denominator {}, only of {}",
                amount.denom,
                staking_info.bonded_denom
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
                    api,
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
                self.validate_denom(&staking_storage, &amount)?;
                self.validate_nonzero(&amount)?;

                // see https://github.com/cosmos/cosmos-sdk/blob/v0.46.1/x/staking/keeper/msg_server.go#L378-L383
                let events = vec![Event::new("unbond")
                    .add_attribute("validator", &validator)
                    .add_attribute("amount", format!("{}{}", amount.amount, amount.denom))
                    .add_attribute("completion_time", "2022-09-27T14:00:00+00:00")]; // TODO: actual date?
                self.remove_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &validator,
                    amount.clone(),
                )?;
                // add tokens to unbonding queue
                let staking_info = Self::get_staking_info(&staking_storage)?;
                UNBONDING_QUEUE.push_back(
                    &mut staking_storage,
                    &(
                        sender.clone(),
                        block.time.plus_seconds(staking_info.unbonding_time),
                        amount.amount.u128(),
                    ),
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
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &src_validator,
                    amount.clone(),
                )?;
                self.add_stake(
                    api,
                    &mut staking_storage,
                    block,
                    &sender,
                    &dst_validator,
                    amount,
                )?;

                Ok(AppResponse { events, data: None })
            }
            m => bail!("Unsupported staking message: {:?}", m),
        }
    }

    fn sudo<ExecC, QueryC: CustomQuery>(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        router: &dyn CosmosRouter<ExecC = ExecC, QueryC = QueryC>,
        block: &BlockInfo,
        msg: StakingSudo,
    ) -> AnyResult<AppResponse> {
        match msg {
            StakingSudo::Slash {
                validator,
                percentage,
            } => {
                let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
                let validator = api.addr_validate(&validator)?;
                self.validate_percentage(percentage)?;

                self.slash(api, &mut staking_storage, block, &validator, percentage)?;

                Ok(AppResponse::default())
            }
            StakingSudo::ProcessQueue {} => {
                loop {
                    let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
                    let front = UNBONDING_QUEUE.front(&staking_storage)?;
                    match front {
                        // assuming the queue is sorted by payout_at
                        Some((_, payout_at, _)) if payout_at <= block.time => {
                            // remove from queue
                            let (delegator, _, amount) =
                                UNBONDING_QUEUE.pop_front(&mut staking_storage)?.unwrap();

                            let staking_info = Self::get_staking_info(&staking_storage)?;
                            router.execute(
                                api,
                                storage,
                                block,
                                self.module_addr.clone(),
                                BankMsg::Send {
                                    to_address: delegator.into_string(),
                                    amount: vec![coin(amount, &staking_info.bonded_denom)],
                                }
                                .into(),
                            )?;
                        }
                        _ => break,
                    }
                }
                Ok(AppResponse::default())
            }
        }
    }

    fn query(
        &self,
        api: &dyn Api,
        storage: &dyn Storage,
        _querier: &dyn Querier,
        block: &BlockInfo,
        request: StakingQuery,
    ) -> AnyResult<Binary> {
        let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);
        match request {
            StakingQuery::BondedDenom {} => Ok(to_binary(&BondedDenomResponse {
                denom: Self::get_staking_info(&staking_storage)?.bonded_denom,
            })?),
            StakingQuery::AllDelegations { delegator } => {
                let delegator = api.addr_validate(&delegator)?;
                let validators = self.get_validators(&staking_storage)?;

                let res: AnyResult<Vec<Delegation>> = validators
                    .into_iter()
                    .filter_map(|validator| {
                        let delegator = delegator.clone();
                        let amount = self
                            .get_stake(
                                &staking_storage,
                                &delegator,
                                &Addr::unchecked(&validator.address),
                            )
                            .transpose()?;

                        Some(amount.map(|amount| Delegation {
                            delegator,
                            validator: validator.address,
                            amount,
                        }))
                    })
                    .collect();

                Ok(to_binary(&AllDelegationsResponse { delegations: res? })?)
            }
            StakingQuery::Delegation {
                delegator,
                validator,
            } => {
                let validator_addr = Addr::unchecked(&validator);
                let validator_obj = match self.get_validator(&staking_storage, &validator_addr)? {
                    Some(validator) => validator,
                    None => bail!("non-existent validator {}", validator),
                };
                let delegator = api.addr_validate(&delegator)?;

                let shares = match STAKES.load(&staking_storage, (&delegator, &validator_addr)) {
                    Ok(stakes) => stakes,
                    Err(_) => {
                        let response = DelegationResponse { delegation: None };
                        return Ok(to_binary(&response)?);
                    }
                };
                let validator_info = VALIDATOR_INFO.load(&staking_storage, &validator_addr)?;
                let reward = Self::get_rewards_internal(
                    &staking_storage,
                    block,
                    &shares,
                    &validator_obj,
                    &validator_info,
                )?;
                let staking_info = Self::get_staking_info(&staking_storage)?;
                let amount = coin(
                    (shares.stake * Uint128::new(1)).u128(),
                    staking_info.bonded_denom,
                );
                let full_delegation_response = DelegationResponse {
                    delegation: Some(FullDelegation {
                        delegator,
                        validator,
                        amount: amount.clone(),
                        can_redelegate: amount, // TODO: not implemented right now
                        accumulated_rewards: if reward.amount.is_zero() {
                            vec![]
                        } else {
                            vec![reward]
                        },
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

    /// Removes all rewards from the given (delegator, validator) pair and returns the amount
    pub fn remove_rewards(
        &self,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        delegator: &Addr,
        validator: &Addr,
    ) -> AnyResult<Uint128> {
        let mut staking_storage = prefixed(storage, NAMESPACE_STAKING);
        // update the validator and staker rewards
        StakeKeeper::update_rewards(api, &mut staking_storage, block, validator)?;

        // load updated rewards for delegator
        let mut shares = STAKES.load(&staking_storage, (delegator, validator))?;
        let rewards = Uint128::new(1) * shares.rewards; // convert to Uint128

        // remove rewards from delegator
        shares.rewards = Decimal::zero();
        STAKES.save(&mut staking_storage, (delegator, validator), &shares)?;

        Ok(rewards)
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
        match msg {
            DistributionMsg::WithdrawDelegatorReward { validator } => {
                let validator_addr = api.addr_validate(&validator)?;

                let rewards = self.remove_rewards(api, storage, block, &sender, &validator_addr)?;

                let staking_storage = prefixed_read(storage, NAMESPACE_STAKING);
                let staking_info = StakeKeeper::get_staking_info(&staking_storage)?;
                // directly mint rewards to delegator
                router.sudo(
                    api,
                    storage,
                    block,
                    BankSudo::Mint {
                        to_address: sender.to_string(),
                        amount: vec![Coin {
                            amount: rewards,
                            denom: staking_info.bonded_denom.clone(),
                        }],
                    }
                    .into(),
                )?;

                let events = vec![Event::new("withdraw_delegator_reward")
                    .add_attribute("validator", &validator)
                    .add_attribute("sender", &sender)
                    .add_attribute(
                        "amount",
                        format!("{}{}", rewards, staking_info.bonded_denom),
                    )];
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
    use crate::{app::MockRouter, BankKeeper, FailingModule, Router, WasmKeeper};

    use super::*;

    use cosmwasm_std::{
        from_slice,
        testing::{mock_env, MockApi, MockStorage},
        BalanceResponse, BankQuery,
    };

    /// Type alias for default build `Router` to make its reference in typical scenario
    type BasicRouter<ExecC = Empty, QueryC = Empty> = Router<
        BankKeeper,
        FailingModule<ExecC, QueryC, Empty>,
        WasmKeeper<ExecC, QueryC>,
        StakeKeeper,
        DistributionKeeper,
    >;

    fn mock_router() -> BasicRouter {
        Router {
            wasm: WasmKeeper::new(),
            bank: BankKeeper::new(),
            custom: FailingModule::new(),
            staking: StakeKeeper::new(),
            distribution: DistributionKeeper::new(),
        }
    }

    fn setup_test_env(
        apr: Decimal,
        validator_commission: Decimal,
    ) -> (MockApi, MockStorage, BasicRouter, BlockInfo, Addr) {
        let api = MockApi::default();
        let router = mock_router();
        let mut store = MockStorage::new();
        let block = mock_env().block;

        let validator = api.addr_validate("testvaloper1").unwrap();

        router
            .staking
            .setup(
                &mut store,
                StakingInfo {
                    bonded_denom: "TOKEN".to_string(),
                    unbonding_time: 60,
                    apr,
                },
            )
            .unwrap();

        // add validator
        let valoper1 = Validator {
            address: "testvaloper1".to_string(),
            commission: validator_commission,
            max_commission: Decimal::percent(100),
            max_change_rate: Decimal::percent(1),
        };
        router
            .staking
            .add_validator(&api, &mut store, &block, valoper1)
            .unwrap();

        (api, store, router, block, validator)
    }

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
                &api,
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
        assert_eq!(
            stake_left.unwrap().amount.u128(),
            50,
            "should have slashed 50%"
        );

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
        assert_eq!(stake_left, None, "should have slashed whole stake");
    }

    #[test]
    fn rewards_work_for_single_delegator() {
        let (api, mut store, router, mut block, validator) =
            setup_test_env(Decimal::percent(10), Decimal::percent(10));
        let stake = &router.staking;
        let distr = &router.distribution;
        let delegator = Addr::unchecked("delegator");

        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        // stake 200 tokens
        stake
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator,
                &validator,
                coin(200, "TOKEN"),
            )
            .unwrap();

        // wait 1/2 year
        block.time = block.time.plus_seconds(60 * 60 * 24 * 365 / 2);

        // should now have 200 * 10% / 2 - 10% commission = 9 tokens reward
        let rewards = stake
            .get_rewards(&store, &block, &delegator, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 9, "should have 9 tokens reward");

        // withdraw rewards
        distr
            .execute(
                &api,
                &mut store,
                &router,
                &block,
                delegator.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator.to_string(),
                },
            )
            .unwrap();

        // should have no rewards left
        let rewards = stake
            .get_rewards(&store, &block, &delegator, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 0);

        // wait another 1/2 year
        block.time = block.time.plus_seconds(60 * 60 * 24 * 365 / 2);
        // should now have 9 tokens again
        let rewards = stake
            .get_rewards(&store, &block, &delegator, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 9);
    }

    #[test]
    fn rewards_work_for_multiple_delegators() {
        let (api, mut store, router, mut block, validator) =
            setup_test_env(Decimal::percent(10), Decimal::percent(10));
        let stake = &router.staking;
        let distr = &router.distribution;
        let bank = &router.bank;
        let delegator1 = Addr::unchecked("delegator1");
        let delegator2 = Addr::unchecked("delegator2");

        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);

        // add 100 stake to delegator1 and 200 to delegator2
        stake
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator1,
                &validator,
                coin(100, "TOKEN"),
            )
            .unwrap();
        stake
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator2,
                &validator,
                coin(200, "TOKEN"),
            )
            .unwrap();

        // wait 1 year
        block.time = block.time.plus_seconds(60 * 60 * 24 * 365);

        // delegator1 should now have 100 * 10% - 10% commission = 9 tokens
        let rewards = stake
            .get_rewards(&store, &block, &delegator1, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 9);

        // delegator2 should now have 200 * 10% - 10% commission = 18 tokens
        let rewards = stake
            .get_rewards(&store, &block, &delegator2, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 18);

        // delegator1 stakes 100 more
        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        stake
            .add_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator1,
                &validator,
                coin(100, "TOKEN"),
            )
            .unwrap();

        // wait another year
        block.time = block.time.plus_seconds(60 * 60 * 24 * 365);

        // delegator1 should now have 9 + 200 * 10% - 10% commission = 27 tokens
        let rewards = stake
            .get_rewards(&store, &block, &delegator1, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 27);

        // delegator2 should now have 18 + 200 * 10% - 10% commission = 36 tokens
        let rewards = stake
            .get_rewards(&store, &block, &delegator2, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 36);

        // delegator2 unstakes 100 (has 100 left after that)
        let mut staking_storage = prefixed(&mut store, NAMESPACE_STAKING);
        stake
            .remove_stake(
                &api,
                &mut staking_storage,
                &block,
                &delegator2,
                &validator,
                coin(100, "TOKEN"),
            )
            .unwrap();

        // and delegator1 withdraws rewards
        distr
            .execute(
                &api,
                &mut store,
                &router,
                &block,
                delegator1.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator.to_string(),
                },
            )
            .unwrap();

        let balance: BalanceResponse = from_slice(
            &bank
                .query(
                    &api,
                    &store,
                    &router.querier(&api, &store, &block),
                    &block,
                    BankQuery::Balance {
                        address: delegator1.to_string(),
                        denom: "TOKEN".to_string(),
                    },
                )
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            balance.amount.amount.u128(),
            27,
            "withdraw should change bank balance"
        );
        let rewards = stake
            .get_rewards(&store, &block, &delegator1, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(
            rewards.amount.u128(),
            0,
            "withdraw should reduce rewards to 0"
        );

        // wait another year
        block.time = block.time.plus_seconds(60 * 60 * 24 * 365);

        // delegator1 should now have 0 + 200 * 10% - 10% commission = 18 tokens
        let rewards = stake
            .get_rewards(&store, &block, &delegator1, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 18);

        // delegator2 should now have 36 + 100 * 10% - 10% commission = 45 tokens
        let rewards = stake
            .get_rewards(&store, &block, &delegator2, &validator)
            .unwrap()
            .unwrap();
        assert_eq!(rewards.amount.u128(), 45);
    }

    mod msg {
        use cosmwasm_std::{from_slice, Addr, BondedDenomResponse, Decimal, StakingQuery};
        use serde::de::DeserializeOwned;

        use super::*;

        // shortens tests a bit
        struct TestEnv {
            api: MockApi,
            store: MockStorage,
            router: BasicRouter,
            block: BlockInfo,
        }

        impl TestEnv {
            fn wrap(tuple: (MockApi, MockStorage, BasicRouter, BlockInfo, Addr)) -> (Self, Addr) {
                (
                    Self {
                        api: tuple.0,
                        store: tuple.1,
                        router: tuple.2,
                        block: tuple.3,
                    },
                    tuple.4,
                )
            }
        }

        fn execute_stake(
            env: &mut TestEnv,
            sender: Addr,
            msg: StakingMsg,
        ) -> AnyResult<AppResponse> {
            env.router.staking.execute(
                &env.api,
                &mut env.store,
                &env.router,
                &env.block,
                sender,
                msg,
            )
        }

        fn query_stake<T: DeserializeOwned>(env: &TestEnv, msg: StakingQuery) -> AnyResult<T> {
            Ok(from_slice(&env.router.staking.query(
                &env.api,
                &env.store,
                &env.router.querier(&env.api, &env.store, &env.block),
                &env.block,
                msg,
            )?)?)
        }

        fn execute_distr(
            env: &mut TestEnv,
            sender: Addr,
            msg: DistributionMsg,
        ) -> AnyResult<AppResponse> {
            env.router.distribution.execute(
                &env.api,
                &mut env.store,
                &env.router,
                &env.block,
                sender,
                msg,
            )
        }

        fn query_bank<T: DeserializeOwned>(env: &TestEnv, msg: BankQuery) -> AnyResult<T> {
            Ok(from_slice(&env.router.bank.query(
                &env.api,
                &env.store,
                &env.router.querier(&env.api, &env.store, &env.block),
                &env.block,
                msg,
            )?)?)
        }

        fn assert_balances(env: &TestEnv, balances: impl IntoIterator<Item = (Addr, u128)>) {
            for (addr, amount) in balances {
                let balance: BalanceResponse = query_bank(
                    env,
                    BankQuery::Balance {
                        address: addr.to_string(),
                        denom: "TOKEN".to_string(),
                    },
                )
                .unwrap();
                assert_eq!(balance.amount.amount.u128(), amount);
            }
        }

        #[test]
        fn execute() {
            // test all execute msgs
            let (mut test_env, validator1) =
                TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

            let delegator1 = Addr::unchecked("delegator1");

            // fund delegator1 account
            test_env
                .router
                .bank
                .init_balance(&mut test_env.store, &delegator1, vec![coin(1000, "TOKEN")])
                .unwrap();

            // add second validator
            let validator2 = Addr::unchecked("validator2");
            test_env
                .router
                .staking
                .add_validator(
                    &test_env.api,
                    &mut test_env.store,
                    &test_env.block,
                    Validator {
                        address: validator2.to_string(),
                        commission: Decimal::zero(),
                        max_commission: Decimal::percent(20),
                        max_change_rate: Decimal::percent(1),
                    },
                )
                .unwrap();

            // delegate 100 tokens to validator1
            execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Delegate {
                    validator: validator1.to_string(),
                    amount: coin(100, "TOKEN"),
                },
            )
            .unwrap();

            // should now have 100 tokens less
            assert_balances(&test_env, vec![(delegator1.clone(), 900)]);

            // wait a year
            test_env.block.time = test_env.block.time.plus_seconds(60 * 60 * 24 * 365);

            // withdraw rewards
            execute_distr(
                &mut test_env,
                delegator1.clone(),
                DistributionMsg::WithdrawDelegatorReward {
                    validator: validator1.to_string(),
                },
            )
            .unwrap();

            // redelegate to validator2
            execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Redelegate {
                    src_validator: validator1.to_string(),
                    dst_validator: validator2.to_string(),
                    amount: coin(100, "TOKEN"),
                },
            )
            .unwrap();

            // should have same amount as before
            assert_balances(
                &test_env,
                vec![(delegator1.clone(), 900 + 100 / 10 * 9 / 10)],
            );

            let delegations: AllDelegationsResponse = query_stake(
                &test_env,
                StakingQuery::AllDelegations {
                    delegator: delegator1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                delegations.delegations,
                [Delegation {
                    delegator: delegator1.clone(),
                    validator: validator2.to_string(),
                    amount: coin(100, "TOKEN"),
                }]
            );

            // undelegate all tokens
            execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Undelegate {
                    validator: validator2.to_string(),
                    amount: coin(100, "TOKEN"),
                },
            )
            .unwrap();

            // wait for unbonding period (60 seconds in default config)
            test_env.block.time = test_env.block.time.plus_seconds(60);

            // need to manually cause queue to get processed
            test_env
                .router
                .staking
                .sudo(
                    &test_env.api,
                    &mut test_env.store,
                    &test_env.router,
                    &test_env.block,
                    StakingSudo::ProcessQueue {},
                )
                .unwrap();

            // check bank balance
            assert_balances(
                &test_env,
                vec![(delegator1.clone(), 1000 + 100 / 10 * 9 / 10)],
            );
        }

        #[test]
        fn cannot_steal() {
            let (mut test_env, validator1) =
                TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));

            let delegator1 = Addr::unchecked("delegator1");

            // fund delegator1 account
            test_env
                .router
                .bank
                .init_balance(&mut test_env.store, &delegator1, vec![coin(100, "TOKEN")])
                .unwrap();

            // delegate 100 tokens to validator1
            execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Delegate {
                    validator: validator1.to_string(),
                    amount: coin(100, "TOKEN"),
                },
            )
            .unwrap();

            // undelegate more tokens than we have
            let e = execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Undelegate {
                    validator: validator1.to_string(),
                    amount: coin(200, "TOKEN"),
                },
            )
            .unwrap_err();

            assert_eq!(e.to_string(), "insufficient stake");

            // add second validator
            let validator2 = Addr::unchecked("validator2");
            test_env
                .router
                .staking
                .add_validator(
                    &test_env.api,
                    &mut test_env.store,
                    &test_env.block,
                    Validator {
                        address: validator2.to_string(),
                        commission: Decimal::zero(),
                        max_commission: Decimal::percent(20),
                        max_change_rate: Decimal::percent(1),
                    },
                )
                .unwrap();

            // redelegate more tokens than we have
            let e = execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Redelegate {
                    src_validator: validator1.to_string(),
                    dst_validator: validator2.to_string(),
                    amount: coin(200, "TOKEN"),
                },
            )
            .unwrap_err();
            assert_eq!(e.to_string(), "insufficient stake");
        }

        #[test]
        fn query_staking() {
            // run all staking queries
            let (mut test_env, validator1) =
                TestEnv::wrap(setup_test_env(Decimal::percent(10), Decimal::percent(10)));
            let delegator1 = Addr::unchecked("delegator1");
            let delegator2 = Addr::unchecked("delegator2");

            // init balances
            test_env
                .router
                .bank
                .init_balance(&mut test_env.store, &delegator1, vec![coin(260, "TOKEN")])
                .unwrap();
            test_env
                .router
                .bank
                .init_balance(&mut test_env.store, &delegator2, vec![coin(150, "TOKEN")])
                .unwrap();

            // add another validator
            let validator2 = test_env.api.addr_validate("testvaloper2").unwrap();
            let valoper2 = Validator {
                address: "testvaloper2".to_string(),
                commission: Decimal::percent(0),
                max_commission: Decimal::percent(1),
                max_change_rate: Decimal::percent(1),
            };
            test_env
                .router
                .staking
                .add_validator(
                    &test_env.api,
                    &mut test_env.store,
                    &test_env.block,
                    valoper2.clone(),
                )
                .unwrap();

            // query validators
            let valoper1: ValidatorResponse = query_stake(
                &test_env,
                StakingQuery::Validator {
                    address: validator1.to_string(),
                },
            )
            .unwrap();
            let validators: AllValidatorsResponse =
                query_stake(&test_env, StakingQuery::AllValidators {}).unwrap();
            assert_eq!(
                validators.validators,
                [valoper1.validator.unwrap(), valoper2]
            );
            // query non-existent validator
            let response = query_stake::<ValidatorResponse>(
                &test_env,
                StakingQuery::Validator {
                    address: "notvaloper".to_string(),
                },
            )
            .unwrap();
            assert_eq!(response.validator, None);

            // query bonded denom
            let response: BondedDenomResponse =
                query_stake(&test_env, StakingQuery::BondedDenom {}).unwrap();
            assert_eq!(response.denom, "TOKEN");

            // delegate some tokens with delegator1 and delegator2
            execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Delegate {
                    validator: validator1.to_string(),
                    amount: coin(100, "TOKEN"),
                },
            )
            .unwrap();
            execute_stake(
                &mut test_env,
                delegator1.clone(),
                StakingMsg::Delegate {
                    validator: validator2.to_string(),
                    amount: coin(160, "TOKEN"),
                },
            )
            .unwrap();
            execute_stake(
                &mut test_env,
                delegator2.clone(),
                StakingMsg::Delegate {
                    validator: validator1.to_string(),
                    amount: coin(150, "TOKEN"),
                },
            )
            .unwrap();

            // query all delegations
            let response1: AllDelegationsResponse = query_stake(
                &test_env,
                StakingQuery::AllDelegations {
                    delegator: delegator1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                response1.delegations,
                vec![
                    Delegation {
                        delegator: delegator1.clone(),
                        validator: validator1.to_string(),
                        amount: coin(100, "TOKEN"),
                    },
                    Delegation {
                        delegator: delegator1.clone(),
                        validator: validator2.to_string(),
                        amount: coin(160, "TOKEN"),
                    },
                ]
            );
            let response2: DelegationResponse = query_stake(
                &test_env,
                StakingQuery::Delegation {
                    delegator: delegator2.to_string(),
                    validator: validator1.to_string(),
                },
            )
            .unwrap();
            assert_eq!(
                response2.delegation.unwrap(),
                FullDelegation {
                    delegator: delegator2.clone(),
                    validator: validator1.to_string(),
                    amount: coin(150, "TOKEN"),
                    accumulated_rewards: vec![],
                    can_redelegate: coin(150, "TOKEN"),
                },
            );
        }
    }
}
