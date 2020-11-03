#[cfg(test)]
use cosmwasm_std::testing::{mock_env, MockApi};
use cosmwasm_std::{
    coin, from_slice, to_binary, to_vec, AllBalanceResponse, BalanceResponse, BankMsg, BankQuery,
    Binary, Coin, HumanAddr, Storage,
};

//*** TODO: remove this and import cw0::balance when we are both on 0.12 ***/
use crate::balance::NativeBalance;

/// Bank is a minimal contract-like interface that implements a bank module
/// It is initialized outside of the trait
pub trait Bank {
    fn handle(
        &self,
        storage: &mut dyn Storage,
        sender: HumanAddr,
        msg: BankMsg,
    ) -> Result<(), String>;

    fn query(&self, storage: &dyn Storage, request: BankQuery) -> Result<Binary, String>;

    // this is an "admin" function to let us adjust bank accounts
    fn set_balance(
        &self,
        storage: &mut dyn Storage,
        account: HumanAddr,
        amount: Vec<Coin>,
    ) -> Result<(), String>;
}

#[derive(Default)]
pub struct SimpleBank {}

impl SimpleBank {
    // this is an "admin" function to let us adjust bank accounts
    pub fn get_balance(
        &self,
        storage: &dyn Storage,
        account: HumanAddr,
    ) -> Result<Vec<Coin>, String> {
        let raw = storage.get(account.as_bytes());
        match raw {
            Some(data) => {
                let balance: NativeBalance = from_slice(&data).map_err(|e| e.to_string())?;
                Ok(balance.into_vec())
            }
            None => Ok(vec![]),
        }
    }

    fn send(
        &self,
        storage: &mut dyn Storage,
        from_address: HumanAddr,
        to_address: HumanAddr,
        amount: Vec<Coin>,
    ) -> Result<(), String> {
        let a = self.get_balance(storage, from_address.clone())?;
        let a = (NativeBalance(a) - amount.clone()).map_err(|e| e.to_string())?;
        self.set_balance(storage, from_address, a.into_vec())?;

        let b = self.get_balance(storage, to_address.clone())?;
        let b = NativeBalance(b) + NativeBalance(amount);
        self.set_balance(storage, to_address, b.into_vec())?;

        Ok(())
    }
}

// TODO: use storage-plus when that is on 0.12.. for now just do this by hand
impl Bank for SimpleBank {
    fn handle(
        &self,
        storage: &mut dyn Storage,
        sender: HumanAddr,
        msg: BankMsg,
    ) -> Result<(), String> {
        match msg {
            BankMsg::Send {
                from_address,
                to_address,
                amount,
            } => {
                if sender != from_address {
                    Err("Sender must equal from_address".into())
                } else {
                    self.send(storage, from_address, to_address, amount)
                }
            }
        }
    }

    fn query(&self, storage: &dyn Storage, request: BankQuery) -> Result<Binary, String> {
        match request {
            BankQuery::AllBalances { address } => {
                let amount = self.get_balance(storage, address)?;
                let res = AllBalanceResponse { amount };
                Ok(to_binary(&res).map_err(|e| e.to_string())?)
            }
            BankQuery::Balance { address, denom } => {
                let all_amounts = self.get_balance(storage, address)?;
                let amount = all_amounts
                    .into_iter()
                    .find(|c| c.denom == denom)
                    .unwrap_or_else(|| coin(0, denom));
                let res = BalanceResponse { amount };
                Ok(to_binary(&res).map_err(|e| e.to_string())?)
            }
        }
    }

    // this is an "admin" function to let us adjust bank accounts
    fn set_balance(
        &self,
        storage: &mut dyn Storage,
        account: HumanAddr,
        amount: Vec<Coin>,
    ) -> Result<(), String> {
        let mut balance = NativeBalance(amount);
        balance.normalize();
        let key = account.as_bytes();
        let value = to_vec(&balance).map_err(|e| e.to_string())?;
        storage.set(key, &value);
        Ok(())
    }
}
