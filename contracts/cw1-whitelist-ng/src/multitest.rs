#[cfg(any(test, feature = "multitest"))]
pub mod contract;

#[cfg(test)]
mod test {
    use crate::contract::Cw1WhitelistContract;
    use crate::msg::whitelist;
    use cosmwasm_std::{to_binary, Addr, CosmosMsg, WasmMsg};
    use cw_multi_test::AppBuilder;

    use super::contract::*;

    #[test]
    fn proxy_freeze_message() {
        let mut app = AppBuilder::new().build(|_, _, _| ());
        let contract_id = app.store_code(Box::new(Cw1WhitelistContract::native()));
        let owner = Addr::unchecked("owner");

        let proxy = Cw1WhitelistProxy::instantiate(&mut app, contract_id, &owner, &[])
            .with_label("Proxy")
            .with_args(vec![owner.to_string()], true)
            .unwrap();

        let remote = Cw1WhitelistProxy::instantiate(&mut app, contract_id, &owner, &[])
            .with_label("Remote")
            .with_args(vec![proxy.addr().into()], true)
            .unwrap();

        assert_ne!(proxy, remote);

        proxy
            .cw1_exec(&mut app, &owner, &[])
            .execute(vec![CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: remote.addr().into(),
                msg: to_binary(&whitelist::ExecMsg::Freeze {}).unwrap(),
                funds: vec![],
            })])
            .unwrap();

        let resp = remote.whitelist_querier(&app.wrap()).admin_list().unwrap();
        assert!(!resp.mutable);
    }
}
