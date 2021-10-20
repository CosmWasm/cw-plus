#[cfg(any(test, feature = "multitest"))]
pub mod contract;

#[cfg(test)]
mod test {
    use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, WasmMsg};
    use cw_multi_test::AppBuilder;

    use crate::msg::ExecuteMsg;
    use crate::state::Cw1WhitelistContract;

    use super::contract::Cw1WhitelistProxy;

    #[test]
    fn proxy_freeze_message() {
        let mut app = AppBuilder::new().build(|_, _, _| ());
        let contract_id = app.store_code(Box::new(Cw1WhitelistContract::new()));
        let owner = Addr::unchecked("owner");

        let proxy = Cw1WhitelistProxy::instantiate(
            &mut app,
            contract_id,
            &owner,
            vec![owner.to_string()],
            true,
            &[],
            "Proxy",
            owner.to_string(),
        )
        .unwrap();

        let remote = Cw1WhitelistProxy::instantiate(
            &mut app,
            contract_id,
            &owner,
            vec![proxy.addr().into()],
            true,
            &[],
            "Remote",
            owner.to_string(),
        )
        .unwrap();

        assert_ne!(proxy, remote);

        proxy
            .execute(
                &mut app,
                &owner,
                vec![CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: remote.addr().into(),
                    msg: to_binary(&ExecuteMsg::<Empty>::Freeze {}).unwrap(),
                    funds: vec![],
                })],
                &[],
            )
            .unwrap();

        let resp = remote
            .cw1_whitelist_querier(&app.wrap())
            .admin_list()
            .unwrap();
        assert!(!resp.mutable);
    }
}
