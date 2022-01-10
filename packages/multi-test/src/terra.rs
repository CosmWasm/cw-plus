use crate::{App, AppResponse, CustomHandler, Router};
use anyhow::{anyhow, Result as AnyResult};
use cosmwasm_std::{
    from_binary, Addr, AllBalanceResponse, Api, BankQuery, Binary, BlockInfo, Coin, Decimal, Event,
    Storage, Uint128,
};
use cw0::NativeBalance;
use terra_cosmwasm::{
    SwapResponse, TaxCapResponse, TaxRateResponse, TerraMsg, TerraMsgWrapper, TerraQuery,
    TerraQueryWrapper, TerraRoute,
};
use terra_mocks::{SwapQuerier, TreasuryQuerier};

pub type TerraApp = App<TerraMsgWrapper, TerraQueryWrapper>;

pub struct TerraMock {
    pub swap: SwapQuerier,
    pub treasury: TreasuryQuerier,
}

impl CustomHandler<TerraMsgWrapper, TerraQueryWrapper> for TerraMock {
    fn execute(
        &self,
        router: &Router<TerraMsgWrapper, TerraQueryWrapper>,
        api: &dyn Api,
        storage: &mut dyn Storage,
        block: &BlockInfo,
        sender: Addr,
        msg: TerraMsgWrapper,
    ) -> AnyResult<AppResponse> {
        match msg.route {
            TerraRoute::Market => match msg.msg_data {
                TerraMsg::Swap {
                    offer_coin,
                    ask_denom,
                } => self.swap(
                    router, api, storage, block, sender, offer_coin, ask_denom, None,
                ),
                TerraMsg::SwapSend {
                    offer_coin,
                    ask_denom,
                    to_address,
                } => self.swap(
                    router,
                    api,
                    storage,
                    block,
                    sender,
                    offer_coin,
                    ask_denom,
                    Some(to_address),
                ),
            },
            _ => {
                panic!("Unexpected custom exec msg {:?} from {:?}", msg, sender)
            }
        }
    }

    fn query(
        &self,
        _api: &dyn Api,
        _storage: &dyn Storage,
        _block: &BlockInfo,
        msg: TerraQueryWrapper,
    ) -> AnyResult<Binary> {
        match msg.route {
            TerraRoute::Market => self.swap.query(&msg.query_data),
            TerraRoute::Treasury => self.treasury.query(&msg.query_data),
            _ => panic!("Unexpected cusom query msg {:?}", msg),
        }
        .into_result()?
        .into_result()
        .map_err(|e| anyhow! {e})
    }

    fn get_taxable_coins(&self, msg: &TerraMsgWrapper) -> Vec<Coin> {
        match msg.route {
            TerraRoute::Market => match &msg.msg_data {
                TerraMsg::SwapSend { offer_coin, .. } => {
                    vec![offer_coin.clone()]
                }
                _ => vec![],
            },
            _ => vec![],
        }
    }

    fn calculate_taxes(&self, coins: &[Coin]) -> AnyResult<Vec<Coin>> {
        let tax_rate: TaxRateResponse = from_binary(
            &self
                .treasury
                .query(&TerraQuery::TaxRate {})
                .into_result()?
                .into_result()
                .map_err(|e| anyhow! {e})?,
        )?;

        let mut result: Vec<Coin> = vec![];

        if !tax_rate.rate.is_zero() {
            for coin in coins {
                let tax_cap: TaxCapResponse = from_binary(
                    &self
                        .treasury
                        .query(&TerraQuery::TaxCap {
                            denom: coin.denom.clone(),
                        })
                        .into_result()?
                        .into_result()
                        .map_err(|e| anyhow! {e})?,
                )?;
                result.push(Coin {
                    denom: coin.denom.clone(),
                    amount: (coin.amount * tax_rate.rate).min(tax_cap.cap),
                });
            }
        }

        Ok(result)
    }
}

impl TerraMock {
    pub fn luna_ust_case() -> Self {
        let swap = SwapQuerier::new(&[
            ("uluna", "uusd", Decimal::from_ratio(77u128, 1u128)),
            ("uusd", "uluna", Decimal::from_ratio(1u128, 77u128)),
        ]);

        let treasury =
            TreasuryQuerier::new(Decimal::from_ratio(1u128, 6u128), &[("uusd", 1_390000)]);

        Self { swap, treasury }
    }

    pub fn swap(
        &self,
        router: &Router<TerraMsgWrapper, TerraQueryWrapper>,
        api: &dyn Api,
        storage: &mut dyn Storage,
        _block: &BlockInfo,
        sender: Addr,
        offer_coin: Coin,
        ask_denom: String,
        to_address: Option<String>,
    ) -> AnyResult<AppResponse> {
        if offer_coin.denom == ask_denom {
            return Err(anyhow! {"recursive swap!"});
        }

        let mut sender_balances = (NativeBalance(
            from_binary::<AllBalanceResponse>(&router.bank.query(
                api,
                storage,
                BankQuery::AllBalances {
                    address: sender.to_string(),
                },
            )?)?
            .amount,
        ) - offer_coin.clone())?;

        let mut ask_receive: Coin = from_binary::<SwapResponse>(
            &self
                .swap
                .query(&TerraQuery::Swap {
                    offer_coin: offer_coin.clone(),
                    ask_denom: ask_denom.clone(),
                })
                .into_result()?
                .into_result()
                .map_err(|e| anyhow! {e})?,
        )?
        .receive;

        // a little spread
        let fee_coin = Coin {
            denom: ask_denom,
            amount: Uint128::new(1),
        };
        ask_receive.amount = ask_receive.amount.saturating_sub(fee_coin.amount);

        let recipient = if let Some(recipient) = to_address {
            let recipient = api.addr_validate(&recipient)?;

            let recipient_balances: Vec<Coin> = (NativeBalance(
                from_binary::<AllBalanceResponse>(&router.bank.query(
                    api,
                    storage,
                    BankQuery::AllBalances {
                        address: recipient.to_string(),
                    },
                )?)?
                .amount,
            ) + ask_receive.clone())
            .into_vec();

            router
                .bank
                .init_balance(storage, &recipient, recipient_balances)?;
            recipient
        } else {
            sender_balances += ask_receive.clone();
            sender.clone()
        };

        router
            .bank
            .init_balance(storage, &sender, sender_balances.into_vec())?;

        let mut response = AppResponse::default();
        response.events.push(
            Event::new("swap")
                .add_attribute("offer", offer_coin.to_string())
                .add_attribute("trader", sender)
                .add_attribute("recipient", recipient)
                .add_attribute("swap_coin", ask_receive.to_string())
                .add_attribute("swap_fee", fee_coin.to_string()),
        );
        response
            .events
            .push(Event::new("message").add_attribute("module", "market"));

        // TODO: we can build response data as here https://github.com/terra-money/core/blob/d6037b9a12c8bf6b09fe861c8ad93456aac5eebb/x/market/keeper/msg_server.go#L144, but it can't be used anywhere in contracts yet

        Ok(response)
    }
}
