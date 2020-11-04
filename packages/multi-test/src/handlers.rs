use std::cell::RefCell;
use std::ops::{Deref, DerefMut};

#[cfg(test)]
use cosmwasm_std::testing::{mock_env, MockApi};
use cosmwasm_std::{
    from_slice, Api, Attribute, BankMsg, BankQuery, Binary, BlockInfo, Coin, ContractResult,
    CosmosMsg, Empty, HandleResponse, HumanAddr, InitResponse, MessageInfo, Querier, QuerierResult,
    QueryRequest, Storage, SystemError, SystemResult, WasmMsg, WasmQuery,
};

use crate::bank::Bank;
use crate::wasm::WasmRouter;
use crate::Contract;

#[derive(Default, Clone, Debug)]
pub struct RouterResponse {
    pub attributes: Vec<Attribute>,
    pub data: Option<Binary>,
}

// This can be InitResponse, HandleResponse, MigrationResponse
#[derive(Default, Clone)]
pub struct ActionResponse {
    // TODO: allow T != Empty
    pub messages: Vec<CosmosMsg<Empty>>,
    pub attributes: Vec<Attribute>,
    pub data: Option<Binary>,
}

impl From<HandleResponse<Empty>> for ActionResponse {
    fn from(input: HandleResponse<Empty>) -> Self {
        ActionResponse {
            messages: input.messages,
            attributes: input.attributes,
            data: input.data,
        }
    }
}

impl ActionResponse {
    fn init(input: InitResponse<Empty>, address: HumanAddr) -> Self {
        ActionResponse {
            messages: input.messages,
            attributes: input.attributes,
            data: Some(address.as_bytes().into()),
        }
    }
}

pub struct Router<S>
where
    S: Storage + Default,
{
    wasm: WasmRouter<S>,
    bank: Box<dyn Bank>,
    bank_store: RefCell<S>,
    // LATER: staking router
}

impl<S> Querier for Router<S>
where
    S: Storage + Default,
{
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<Empty> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e.to_string()),
                    request: bin_request.into(),
                })
            }
        };
        let contract_result: ContractResult<Binary> = self.query(request).into();
        SystemResult::Ok(contract_result)
    }
}

impl<S> Router<S>
where
    S: Storage + Default,
{
    // TODO: store BlockInfo in Router not WasmRouter to change easier?
    pub fn new<B: Bank + 'static>(api: Box<dyn Api>, block: BlockInfo, bank: B) -> Self {
        Router {
            wasm: WasmRouter::new(api, block),
            bank: Box::new(bank),
            bank_store: RefCell::new(S::default()),
        }
    }

    pub fn set_block(&mut self, block: BlockInfo) {
        self.wasm.set_block(block);
    }

    // this let's use use "next block" steps that add eg. one height and 5 seconds
    pub fn update_block<F: Fn(&mut BlockInfo)>(&mut self, action: F) {
        self.wasm.update_block(action);
    }

    // this is an "admin" function to let us adjust bank accounts
    pub fn set_bank_balance(&self, account: HumanAddr, amount: Vec<Coin>) -> Result<(), String> {
        let mut store = self
            .bank_store
            .try_borrow_mut()
            .map_err(|e| format!("Double-borrowing mutable storage - re-entrancy?: {}", e))?;
        self.bank.set_balance(store.deref_mut(), account, amount)
    }

    pub fn store_code(&mut self, code: Box<dyn Contract>) -> u64 {
        self.wasm.store_code(code) as u64
    }

    pub fn execute(
        &mut self,
        sender: HumanAddr,
        msg: CosmosMsg<Empty>,
    ) -> Result<RouterResponse, String> {
        // TODO: we need to do some caching of storage here, once in the entry point
        // meaning, wrap current state.. all writes go to a cache... only when execute
        // returns a success do we flush it (otherwise drop it)
        self._execute(&sender, msg)
    }

    fn _execute(
        &mut self,
        sender: &HumanAddr,
        msg: CosmosMsg<Empty>,
    ) -> Result<RouterResponse, String> {
        match msg {
            CosmosMsg::Wasm(msg) => {
                let (resender, res) = self.handle_wasm(sender, msg)?;
                let mut attributes = res.attributes;
                // recurse in all messages
                for resend in res.messages {
                    let subres = self._execute(&resender, resend)?;
                    // ignore the data now, just like in wasmd
                    // append the events
                    attributes.extend_from_slice(&subres.attributes);
                }
                Ok(RouterResponse {
                    attributes,
                    data: res.data,
                })
            }
            CosmosMsg::Bank(msg) => self.handle_bank(sender, msg),
            _ => unimplemented!(),
        }
    }

    fn handle_wasm(
        &mut self,
        sender: &HumanAddr,
        msg: WasmMsg,
    ) -> Result<(HumanAddr, ActionResponse), String> {
        match msg {
            WasmMsg::Execute {
                contract_addr,
                msg,
                send,
            } => {
                // first move the cash
                self.send(sender, &contract_addr, &send)?;
                // then call the contract
                let info = MessageInfo {
                    sender: sender.clone(),
                    sent_funds: send,
                };
                let res = self
                    .wasm
                    .handle(contract_addr.clone(), self, info, msg.to_vec())?;
                Ok((contract_addr, res.into()))
            }
            WasmMsg::Instantiate {
                code_id,
                msg,
                send,
                label: _,
            } => {
                // register the contract
                let contract_addr = self.wasm.register_contract(code_id as usize)?;
                // move the cash
                self.send(sender, &contract_addr, &send)?;
                // then call the contract
                let info = MessageInfo {
                    sender: sender.clone(),
                    sent_funds: send,
                };
                let res = self
                    .wasm
                    .init(contract_addr.clone(), self, info, msg.to_vec())?;
                Ok((
                    contract_addr.clone(),
                    ActionResponse::init(res, contract_addr),
                ))
            }
        }
    }

    // Returns empty router response, just here for the same function signatures
    fn handle_bank(&self, sender: &HumanAddr, msg: BankMsg) -> Result<RouterResponse, String> {
        let mut store = self
            .bank_store
            .try_borrow_mut()
            .map_err(|e| format!("Double-borrowing mutable storage - re-entrancy?: {}", e))?;
        self.bank.handle(store.deref_mut(), sender.into(), msg)?;
        Ok(RouterResponse::default())
    }

    fn send<T: Into<HumanAddr>, U: Into<HumanAddr>>(
        &self,
        sender: T,
        recipient: U,
        amount: &[Coin],
    ) -> Result<RouterResponse, String> {
        if !amount.is_empty() {
            let sender: HumanAddr = sender.into();
            self.handle_bank(
                &sender,
                BankMsg::Send {
                    from_address: sender.clone(),
                    to_address: recipient.into(),
                    amount: amount.to_vec(),
                },
            )?;
        }
        Ok(RouterResponse::default())
    }

    pub fn query(&self, request: QueryRequest<Empty>) -> Result<Binary, String> {
        match request {
            QueryRequest::Wasm(req) => self.query_wasm(req),
            QueryRequest::Bank(req) => self.query_bank(req),
            _ => unimplemented!(),
        }
    }

    fn query_wasm(&self, request: WasmQuery) -> Result<Binary, String> {
        match request {
            WasmQuery::Smart { contract_addr, msg } => {
                self.wasm.query(contract_addr, self, msg.into())
            }
            WasmQuery::Raw { contract_addr, key } => self.wasm.query_raw(contract_addr, &key),
        }
    }

    fn query_bank(&self, request: BankQuery) -> Result<Binary, String> {
        let store = self
            .bank_store
            .try_borrow()
            .map_err(|e| format!("Immutable storage borrow failed - re-entrancy?: {}", e))?;
        self.bank.query(store.deref(), request)
    }
}

// this parses the result from a wasm contract init
pub fn parse_contract_addr(data: &Option<Binary>) -> Result<HumanAddr, String> {
    let bin = data
        .as_ref()
        .ok_or_else(|| "No data response".to_string())?
        .to_vec();
    let str = String::from_utf8(bin).map_err(|e| e.to_string())?;
    Ok(HumanAddr::from(str))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test_helpers::{contract_payout, PayoutMessage};
    use crate::SimpleBank;
    use cosmwasm_std::testing::MockStorage;
    use cosmwasm_std::{attr, coin, coins, to_binary, QuerierWrapper};

    fn mock_router() -> Router<MockStorage> {
        let env = mock_env();
        let api = Box::new(MockApi::default());
        let bank = SimpleBank {};

        Router::new(api, env.block, bank)
    }

    fn get_balance(router: &Router<MockStorage>, addr: &HumanAddr) -> Vec<Coin> {
        QuerierWrapper::new(router)
            .query_all_balances(addr)
            .unwrap()
    }

    #[test]
    fn send_tokens() {
        let mut router = mock_router();

        let owner = HumanAddr::from("owner");
        let rcpt = HumanAddr::from("receiver");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        let rcpt_funds = vec![coin(5, "btc")];

        // set money
        router
            .set_bank_balance(owner.clone(), init_funds.clone())
            .unwrap();
        router
            .set_bank_balance(rcpt.clone(), rcpt_funds.clone())
            .unwrap();

        // send both tokens
        let to_send = vec![coin(30, "eth"), coin(5, "btc")];
        let msg: CosmosMsg = BankMsg::Send {
            from_address: owner.clone(),
            to_address: rcpt.clone(),
            amount: to_send.clone(),
        }
        .into();
        router.execute(owner.clone(), msg.clone()).unwrap();
        let rich = get_balance(&router, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
        let poor = get_balance(&router, &rcpt);
        assert_eq!(vec![coin(10, "btc"), coin(30, "eth")], poor);

        // cannot send from other account
        router.execute(rcpt.clone(), msg).unwrap_err();

        // cannot send too much
        let msg = BankMsg::Send {
            from_address: owner.clone(),
            to_address: rcpt.clone(),
            amount: coins(20, "btc"),
        }
        .into();
        router.execute(owner.clone(), msg).unwrap_err();

        let rich = get_balance(&router, &owner);
        assert_eq!(vec![coin(15, "btc"), coin(70, "eth")], rich);
    }

    #[test]
    fn simple_contract() {
        let mut router = mock_router();
        let code_id = router.store_code(contract_payout());

        // set personal balance
        let owner = HumanAddr::from("owner");
        let init_funds = vec![coin(20, "btc"), coin(100, "eth")];
        router
            .set_bank_balance(owner.clone(), init_funds.clone())
            .unwrap();

        // TODO: add helper to router to set up contract
        // instantiate contract
        let init_msg = to_binary(&PayoutMessage {
            payout: coin(5, "eth"),
        })
        .unwrap();
        let msg: CosmosMsg = WasmMsg::Instantiate {
            code_id,
            msg: init_msg,
            send: coins(23, "eth"),
            label: Some("Payout".to_string()),
        }
        .into();
        let res = router.execute(owner.clone(), msg).unwrap();
        // deduct funds
        let sender = get_balance(&router, &owner);
        assert_eq!(sender, vec![coin(20, "btc"), coin(77, "eth")]);

        // get contract address, has funds
        let contract_addr = parse_contract_addr(&res.data).unwrap();
        let funds = get_balance(&router, &contract_addr);
        assert_eq!(funds, coins(23, "eth"));

        // do one payout and see money coming in
        let random = HumanAddr::from("random");
        let funds = get_balance(&router, &random);
        assert_eq!(funds, vec![]);

        let msg = WasmMsg::Execute {
            contract_addr: contract_addr.clone(),
            msg: b"{}".into(),
            send: vec![],
        }
        .into();
        let res = router.execute(random.clone(), msg).unwrap();
        assert_eq!(1, res.attributes.len());
        assert_eq!(&attr("action", "payout"), &res.attributes[0]);

        // random got cash
        let funds = get_balance(&router, &random);
        assert_eq!(funds, coins(5, "eth"));
        // contract lost it
        let funds = get_balance(&router, &contract_addr);
        assert_eq!(funds, coins(18, "eth"));
    }
}
