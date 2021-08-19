use cosmwasm_std::{
    coin, to_binary, Addr, AllBalanceResponse, Api, BalanceResponse, BankMsg, BankQuery, Binary,
    Coin, Event, Storage,
};

use crate::executor::AppResponse;
use cosmwasm_storage::{prefixed, prefixed_read};
use cw0::NativeBalance;
use cw_storage_plus::Map;
use itertools::Itertools;

use anyhow::{bail, Result as AnyResult};

const BALANCES: Map<&Addr, NativeBalance> = Map::new("balances");

pub const NAMESPACE_BANK: &[u8] = b"bank";

/// Bank is a minimal contract-like interface that implements a bank module
/// It is initialized outside of the trait
pub trait Bank {
    fn execute(
        &self,
        storage: &mut dyn Storage,
        sender: Addr,
        msg: BankMsg,
    ) -> AnyResult<AppResponse>;

    fn query(&self, api: &dyn Api, storage: &dyn Storage, request: BankQuery) -> AnyResult<Binary>;

    // Admin interface
    fn init_balance(
        &self,
        storage: &mut dyn Storage,
        account: &Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()>;
}

#[derive(Default)]
pub struct BankKeeper {}

impl BankKeeper {
    pub fn new() -> Self {
        BankKeeper {}
    }

    fn set_balance(
        &self,
        bank_storage: &mut dyn Storage,
        account: &Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let mut balance = NativeBalance(amount);
        balance.normalize();
        BALANCES
            .save(bank_storage, account, &balance)
            .map_err(Into::into)
    }

    // this is an "admin" function to let us adjust bank accounts
    fn get_balance(&self, bank_storage: &dyn Storage, account: &Addr) -> AnyResult<Vec<Coin>> {
        let val = BALANCES.may_load(bank_storage, &account)?;
        Ok(val.unwrap_or_default().into_vec())
    }

    fn send(
        &self,
        bank_storage: &mut dyn Storage,
        from_address: Addr,
        to_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        self.burn(bank_storage, from_address, amount.clone())?;
        self.mint(bank_storage, to_address, amount)
    }

    fn mint(
        &self,
        bank_storage: &mut dyn Storage,
        to_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let b = self.get_balance(bank_storage, &to_address)?;
        let b = NativeBalance(b) + NativeBalance(amount);
        self.set_balance(bank_storage, &to_address, b.into_vec())
    }

    fn burn(
        &self,
        bank_storage: &mut dyn Storage,
        from_address: Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let a = self.get_balance(bank_storage, &from_address)?;
        let a = (NativeBalance(a) - amount)?;
        self.set_balance(bank_storage, &from_address, a.into_vec())
    }
}

fn coins_to_string(coins: &[Coin]) -> String {
    coins
        .iter()
        .map(|c| format!("{}{}", c.amount, c.denom))
        .join(",")
}

impl Bank for BankKeeper {
    fn execute(
        &self,
        storage: &mut dyn Storage,
        sender: Addr,
        msg: BankMsg,
    ) -> AnyResult<AppResponse> {
        let mut bank_storage = prefixed(storage, NAMESPACE_BANK);
        match msg {
            BankMsg::Send { to_address, amount } => {
                // see https://github.com/cosmos/cosmos-sdk/blob/v0.42.7/x/bank/keeper/send.go#L142-L147
                let events = vec![Event::new("transfer")
                    .add_attribute("recipient", &to_address)
                    .add_attribute("sender", &sender)
                    .add_attribute("amount", coins_to_string(&amount))];
                self.send(
                    &mut bank_storage,
                    sender,
                    Addr::unchecked(to_address),
                    amount,
                )?;
                Ok(AppResponse { events, data: None })
            }
            BankMsg::Burn { amount } => {
                // burn doesn't seem to emit any events
                self.burn(&mut bank_storage, sender, amount)?;
                Ok(AppResponse::default())
            }
            m => bail!("Unsupported bank message: {:?}", m),
        }
    }

    fn query(&self, api: &dyn Api, storage: &dyn Storage, request: BankQuery) -> AnyResult<Binary> {
        let bank_storage = prefixed_read(storage, NAMESPACE_BANK);
        match request {
            BankQuery::AllBalances { address } => {
                let address = api.addr_validate(&address)?;
                let amount = self.get_balance(&bank_storage, &address)?;
                let res = AllBalanceResponse { amount };
                Ok(to_binary(&res)?)
            }
            BankQuery::Balance { address, denom } => {
                let address = api.addr_validate(&address)?;
                let all_amounts = self.get_balance(&bank_storage, &address)?;
                let amount = all_amounts
                    .into_iter()
                    .find(|c| c.denom == denom)
                    .unwrap_or_else(|| coin(0, denom));
                let res = BalanceResponse { amount };
                Ok(to_binary(&res)?)
            }
            q => bail!("Unsupported bank query: {:?}", q),
        }
    }

    // this is an "admin" function to let us adjust bank accounts
    fn init_balance(
        &self,
        storage: &mut dyn Storage,
        account: &Addr,
        amount: Vec<Coin>,
    ) -> AnyResult<()> {
        let mut bank_storage = prefixed(storage, NAMESPACE_BANK);
        self.set_balance(&mut bank_storage, account, amount)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use cosmwasm_std::testing::{MockApi, MockStorage};
    use cosmwasm_std::{coins, from_slice, StdError};

    fn query_balance(
        bank: &BankKeeper,
        api: &dyn Api,
        store: &dyn Storage,
        rcpt: &Addr,
    ) -> Vec<Coin> {
        let req = BankQuery::AllBalances {
            address: rcpt.clone().into(),
        };
        let raw = bank.query(api, store, req).unwrap();
        let res: AllBalanceResponse = from_slice(&raw).unwrap();
        res.amount
    }

    #[test]
    fn get_set_balance() {
        let api = MockApi::default();
        let mut store = MockStorage::new();

        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("receiver");
        let init_funds = vec![coin(100, "eth"), coin(20, "btc")];
        let norm = vec![coin(20, "btc"), coin(100, "eth")];

        // set money
        let bank = BankKeeper::new();
        bank.init_balance(&mut store, &owner, init_funds).unwrap();
        let bank_storage = prefixed_read(&store, NAMESPACE_BANK);

        // get balance work
        let rich = bank.get_balance(&bank_storage, &owner).unwrap();
        assert_eq!(rich, norm);
        let poor = bank.get_balance(&bank_storage, &rcpt).unwrap();
        assert_eq!(poor, vec![]);

        // proper queries work
        let req = BankQuery::AllBalances {
            address: owner.clone().into(),
        };
        let raw = bank.query(&api, &store, req).unwrap();
        let res: AllBalanceResponse = from_slice(&raw).unwrap();
        assert_eq!(res.amount, norm);

        let req = BankQuery::AllBalances {
            address: rcpt.clone().into(),
        };
        let raw = bank.query(&api, &store, req).unwrap();
        let res: AllBalanceResponse = from_slice(&raw).unwrap();
        assert_eq!(res.amount, vec![]);

        let req = BankQuery::Balance {
            address: owner.clone().into(),
            denom: "eth".into(),
        };
        let raw = bank.query(&api, &store, req).unwrap();
        let res: BalanceResponse = from_slice(&raw).unwrap();
        assert_eq!(res.amount, coin(100, "eth"));

        let req = BankQuery::Balance {
            address: owner.into(),
            denom: "foobar".into(),
        };
        let raw = bank.query(&api, &store, req).unwrap();
        let res: BalanceResponse = from_slice(&raw).unwrap();
        assert_eq!(res.amount, coin(0, "foobar"));

        let req = BankQuery::Balance {
            address: rcpt.into(),
            denom: "eth".into(),
        };
        let raw = bank.query(&api, &store, req).unwrap();
        let res: BalanceResponse = from_slice(&raw).unwrap();
        assert_eq!(res.amount, coin(0, "eth"));
    }

    #[test]
    fn send_coins() {
        let api = MockApi::default();
        let mut store = MockStorage::new();

        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("receiver");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        let rcpt_funds = vec![coin(5, "btc")];

        // set money
        let bank = BankKeeper::new();
        bank.init_balance(&mut store, &owner, init_funds).unwrap();
        bank.init_balance(&mut store, &rcpt, rcpt_funds).unwrap();

        // send both tokens
        let to_send = vec![coin(30, "eth"), coin(5, "btc")];
        let msg = BankMsg::Send {
            to_address: rcpt.clone().into(),
            amount: to_send,
        };
        bank.execute(&mut store, owner.clone(), msg.clone())
            .unwrap();
        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
        let poor = query_balance(&bank, &api, &store, &rcpt);
        assert_eq!(vec![coin(10, "btc"), coin(30, "eth")], poor);

        // can send from any account with funds
        bank.execute(&mut store, rcpt.clone(), msg).unwrap();

        // cannot send too much
        let msg = BankMsg::Send {
            to_address: rcpt.into(),
            amount: coins(20, "btc"),
        };
        bank.execute(&mut store, owner.clone(), msg).unwrap_err();

        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
    }

    #[test]
    fn burn_coins() {
        let api = MockApi::default();
        let mut store = MockStorage::new();

        let owner = Addr::unchecked("owner");
        let rcpt = Addr::unchecked("recipient");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];

        // set money
        let bank = BankKeeper::new();
        bank.init_balance(&mut store, &owner, init_funds).unwrap();

        // burn both tokens
        let to_burn = vec![coin(30, "eth"), coin(5, "btc")];
        let msg = BankMsg::Burn { amount: to_burn };
        bank.execute(&mut store, owner.clone(), msg).unwrap();
        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);

        // cannot burn too much
        let msg = BankMsg::Burn {
            amount: coins(20, "btc"),
        };
        let err = bank.execute(&mut store, owner.clone(), msg).unwrap_err();
        assert!(matches!(err.downcast().unwrap(), StdError::Overflow { .. }));

        let rich = query_balance(&bank, &api, &store, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);

        // cannot burn from empty account
        let msg = BankMsg::Burn {
            amount: coins(1, "btc"),
        };
        let err = bank.execute(&mut store, rcpt, msg).unwrap_err();
        assert!(matches!(err.downcast().unwrap(), StdError::Overflow { .. }));
    }
}
