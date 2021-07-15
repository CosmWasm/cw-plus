use crate::contract::{handle, init, query};
use anchor_token::airdrop::{
    ConfigResponse, HandleMsg, InitMsg, IsClaimedResponse, LatestStageResponse, MerkleRootResponse,
    QueryMsg,
};
use cosmwasm_std::testing::{mock_dependencies, mock_env};
use cosmwasm_std::{from_binary, log, to_binary, CosmosMsg, HumanAddr, StdError, Uint128, WasmMsg};
use cw20::Cw20HandleMsg;

#[test]
fn proper_initialization() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr("owner0000".to_string()),
        anchor_token: HumanAddr("anchor0000".to_string()),
    };

    let env = mock_env("addr0000", &[]);

    // we can just call .unwrap() to assert this was a success
    let _res = init(&mut deps, env, msg).unwrap();

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner0000", config.owner.as_str());
    assert_eq!("anchor0000", config.anchor_token.as_str());

    let res = query(&deps, QueryMsg::LatestStage {}).unwrap();
    let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
    assert_eq!(0u8, latest_stage.latest_stage);
}

#[test]
fn update_config() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        anchor_token: HumanAddr::from("anchor0000"),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // update owner
    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig {
        owner: Some(HumanAddr("owner0001".to_string())),
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = query(&deps, QueryMsg::Config {}).unwrap();
    let config: ConfigResponse = from_binary(&res).unwrap();
    assert_eq!("owner0001", config.owner.as_str());

    // Unauthorzied err
    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::UpdateConfig { owner: None };

    let res = handle(&mut deps, env, msg);
    match res {
        Err(StdError::Unauthorized { .. }) => {}
        _ => panic!("Must return unauthorized error"),
    }
}

#[test]
fn register_merkle_root() {
    let mut deps = mock_dependencies(20, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        anchor_token: HumanAddr::from("anchor0000"),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env.clone(), msg).unwrap();

    // register new merkle root
    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
    };

    let res = handle(&mut deps, env, msg).unwrap();
    assert_eq!(
        res.log,
        vec![
            log("action", "register_merkle_root"),
            log("stage", "1"),
            log(
                "merkle_root",
                "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
            )
        ]
    );

    let res = query(&deps, QueryMsg::LatestStage {}).unwrap();
    let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
    assert_eq!(1u8, latest_stage.latest_stage);

    let res = query(
        &deps,
        QueryMsg::MerkleRoot {
            stage: latest_stage.latest_stage,
        },
    )
    .unwrap();
    let merkle_root: MerkleRootResponse = from_binary(&res).unwrap();
    assert_eq!(
        "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
        merkle_root.merkle_root
    );
}

#[test]
fn claim() {
    let mut deps = mock_dependencies(44, &[]);

    let msg = InitMsg {
        owner: HumanAddr::from("owner0000"),
        anchor_token: HumanAddr::from("anchor0000"),
    };

    let env = mock_env("addr0000", &[]);
    let _res = init(&mut deps, env, msg).unwrap();

    // Register merkle roots
    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "85e33930e7a8f015316cb4a53a4c45d26a69f299fc4c83f17357e1fd62e8fd95".to_string(),
    };
    let _res = handle(&mut deps, env, msg).unwrap();

    let env = mock_env("owner0000", &[]);
    let msg = HandleMsg::RegisterMerkleRoot {
        merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
    };
    let _res = handle(&mut deps, env, msg).unwrap();

    let msg = HandleMsg::Claim {
        amount: Uint128::from(1000001u128),
        stage: 1u8,
        proof: vec![
            "b8ee25ffbee5ee215c4ad992fe582f20175868bc310ad9b2b7bdf440a224b2df".to_string(),
            "98d73e0a035f23c490fef5e307f6e74652b9d3688c2aa5bff70eaa65956a24e1".to_string(),
            "f328b89c766a62b8f1c768fefa1139c9562c6e05bab57a2af87f35e83f9e9dcf".to_string(),
            "fe19ca2434f87cadb0431311ac9a484792525eb66a952e257f68bf02b4561950".to_string(),
        ],
    };

    let env = mock_env(
        "terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8".to_string(),
        &[],
    );
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("anchor0000"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                amount: Uint128::from(1000001u128),
            })
            .unwrap(),
        })]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "claim"),
            log("stage", "1"),
            log("address", "terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
            log("amount", "1000001")
        ]
    );

    assert_eq!(
        true,
        from_binary::<IsClaimedResponse>(
            &query(
                &mut deps,
                QueryMsg::IsClaimed {
                    stage: 1,
                    address: HumanAddr::from("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                }
            )
            .unwrap()
        )
        .unwrap()
        .is_claimed
    );

    let res = handle(&mut deps, env.clone(), msg.clone());
    match res {
        Err(StdError::GenericErr { msg, .. }) => assert_eq!(msg, "Already claimed"),
        _ => panic!("DO NOT ENTER HERE"),
    }

    // Claim next airdrop
    let msg = HandleMsg::Claim {
        amount: Uint128::from(2000001u128),
        stage: 2u8,
        proof: vec![
            "ca2784085f944e5594bb751c3237d6162f7c2b24480b3a37e9803815b7a5ce42".to_string(),
            "5b07b5898fc9aa101f27344dab0737aede6c3aa7c9f10b4b1fda6d26eb669b0f".to_string(),
            "4847b2b9a6432a7bdf2bdafacbbeea3aab18c524024fc6e1bc655e04cbc171f3".to_string(),
            "cad1958c1a5c815f23450f1a2761a5a75ab2b894a258601bf93cd026469d42f2".to_string(),
        ],
    };

    let env = mock_env(
        "terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8".to_string(),
        &[],
    );
    let res = handle(&mut deps, env.clone(), msg.clone()).unwrap();
    assert_eq!(
        res.messages,
        vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: HumanAddr::from("anchor0000"),
            send: vec![],
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: HumanAddr::from("terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
                amount: Uint128::from(2000001u128),
            })
            .unwrap(),
        })]
    );

    assert_eq!(
        res.log,
        vec![
            log("action", "claim"),
            log("stage", "2"),
            log("address", "terra1qfqa2eu9wp272ha93lj4yhcenrc6ymng079nu8"),
            log("amount", "2000001")
        ]
    );
}
