#![cfg(test)]

use cosmwasm_std::{coins, Addr, Coin, Uint128};
use fanfuryswap::msg::ConfigResponse as FanfuryConfigResponse;

use crate::{
    error::ContractError,
    msg::{BondStateResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
};

use cw_multi_test::{App, Contract, ContractWrapper, Executor};

fn mock_app() -> App {
    App::default()
}

pub fn contract_bonding() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );

    Box::new(contract)
}

pub fn contract_amm() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        fanfuryswap::contract::execute,
        fanfuryswap::contract::instantiate,
        fanfuryswap::contract::query,
    )
    .with_reply(fanfuryswap::contract::reply);
    Box::new(contract)
}

fn get_config(router: &App, contract_addr: &Addr) -> ConfigResponse {
    router
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::Config {})
        .unwrap()
}

fn get_pool_config(router: &App, contract_addr: &Addr) -> FanfuryConfigResponse {
    router
        .wrap()
        .query_wasm_smart(contract_addr, &fanfuryswap::msg::QueryMsg::Config {})
        .unwrap()
}

fn get_pool_address(router: &App, contract_addr: &Addr) -> Addr {
    get_config(router, contract_addr).pool_address
}

fn get_bonding_info(router: &App, contract_addr: &Addr, user: &Addr) -> BondStateResponse {
    router
        .wrap()
        .query_wasm_smart(contract_addr, &QueryMsg::BondState {
            address: user.clone(),
        })
        .unwrap()
}

fn get_pair_bonding_contract(router: &App, contract_addr: &Addr, user: &Addr) -> Addr {
    let res: FanfuryConfigResponse = router
        .wrap()
        .query_wasm_smart(contract_addr, &fanfuryswap::msg::QueryMsg::Config {})
        .unwrap();

    res.bonding_contract_address
}

fn create_amm(router: &mut App, owner: &Addr, cash: &Addr, native_denom: &str) -> Addr {
    let amm_id = router.store_code(contract_amm());
    let bonding_id = router.store_code(contract_bonding());
    let msg = fanfuryswap::msg::InstantiateMsg {
        lp_token_code_id: amm_id,
        bonding_code_id: bonding_id,
        owner: owner.clone(),
        treasury_address: owner.clone(),
        fury_token_address: cash.clone(),
        usdc_denom: native_denom.to_string(),
        lock_seconds: 7u64,
        discount: 5u64,
        tx_fee: 3u64,
        platform_fee: 10u64,
        daily_vesting_amount: Uint128::from(10000000000u128),
    };
    router
        .instantiate_contract(amm_id, owner.clone(), &msg, &[], "amm", None)
        .unwrap()
}

fn create_native_bonding(
    router: &mut App,
    owner: &Addr,
    pool_address: &Addr,
    cash: &Addr,
    native_denom: &str,
) -> Addr {
    let bonding_id = router.store_code(contract_bonding());
    let msg = InstantiateMsg {
        owner: owner.clone(),
        pool_address: pool_address.clone(),
        treasury_address: owner.clone(),
        fury_token_address: cash.clone(),
        usdc_denom: native_denom.to_string(),
        lock_seconds: 5u64,
        discount: 7u64,
        tx_fee: 3u64,
        platform_fee: 10u64,
        daily_vesting_amount: Uint128::from(10000000000u128),
        is_native_bonding: true,
    };
    router
        .instantiate_contract(bonding_id, owner.clone(), &msg, &[], "nativebond", None)
        .unwrap()
}

fn create_token(
    router: &mut App,
    owner: &Addr,
    name: String,
    symbol: String,
    total_supply: Uint128,
) -> Addr {
    let token_id = router.store_code(contract_token());
    let msg = tokenfactory::msg::InstantiateMsg {
        name,
        symbol,
        decimals: 6,
        initial_balances: vec![],
        mint: None,
        marketing: None,
        treasury: None,
    };
    router
        .instantiate_contract(token_id, owner.clone(), &msg, &[], "token", None)
        .unwrap()
}

fn bank_balance(router: &mut App, addr: &Addr, denom: &str) -> Coin {
    router
        .wrap()
        .query_balance(addr.to_string(), denom.to_string())
        .unwrap()
}

#[test]
// receive native tokens and release upon approval
fn test_instantiate() {
    let mut router = mock_app();

    const NATIVE_TOKEN_DENOM: &str = "usdc";

    let owner = Addr::unchecked("owner");
    let funds = coins(2000000, NATIVE_TOKEN_DENOM);
    router
        .borrow_mut()
        .init_modules(|router, _, storage| router.bank.init_balance(storage, &owner, funds).unwrap());

    let native_token = create_token(
        &mut router,
        &owner,
        "fury token".to_string(),
        "FURY".to_string(),
        Uint128::new(2000000),
    );

    let amm_addr = create_amm(&mut router, &owner, &native_token, NATIVE_TOKEN_DENOM);
    let native_bonding_addr =
        create_native_bonding(&mut router, &owner, &amm_addr, &native_token, NATIVE_TOKEN_DENOM);

    assert_ne!(native_token, amm_addr);

    let info = get_config(&router, &native_bonding_addr);
    println!("{:?}", info);
    assert_eq!(info.pool_address, "contract1".to_string());
}

#[test]
fn lp_bonding() {
    let mut router = mock_app();

    const NATIVE_TOKEN_DENOM: &str = "usdc";

    let owner = Addr::unchecked("owner");
    let bonder = Addr::unchecked("bonder");
    let funds = coins(200000, NATIVE_TOKEN_DENOM);

    router
        .borrow_mut()
        .init_modules(|router, _, storage| router.bank.init_balance(storage, &bonder, funds).unwrap());

    let token = create_token(
        &mut router,
        &bonder,
        "fury token".to_string(),
        "FURY".to_string(),
        Uint128::new(500000),
    );

    let amm = create_amm(&mut router, &owner, &token, NATIVE_TOKEN_DENOM);

    // Add initial liquidity to pools
    // Simulate increase allowance
    let add_liquidity_msg = fanfuryswap::msg::ExecuteMsg::AddLiquidity {
        token1_amount: Uint128::new(100000),
        min_liquidity: Uint128::new(100000),
        max_token2: Uint128::new(100000),
        fee_amount: Uint128::new(2600),
        expiration: None,
    };
    let _res = router
        .execute_contract(
            bonder.clone(),
            amm.clone(),
            &add_liquidity_msg,
            &[Coin {
                denom: NATIVE_TOKEN_DENOM.into(),
                amount: Uint128::new(102600),
            }],
        )
        .unwrap();

    // Ensure balances updated
    let token1_balance = bank_balance(&mut router, &bonder, NATIVE_TOKEN_DENOM);
    assert_eq!(token1_balance, Uint128::new(400000));

    // Check pool contract info
    let res: FanfuryConfigResponse = get_pool_config(&router, &amm);
    let bonding_contract = res.bonding_contract_address;

    // Check bonding record
    let record = get_bonding_info(&router, &bonding_contract, &bonder);
    assert_eq!(record.list[0].amount, Uint128::new(201005));
}

#[test]
fn native_bonding() {
    let mut router = mock_app();

    const NATIVE_TOKEN_DENOM: &str = "usdc";

    let owner = Addr::unchecked("owner");
    let bonder = Addr::unchecked("bonder");
    let funds = coins(200000, NATIVE_TOKEN_DENOM);

    router
        .borrow_mut()
        .init_modules(|router, _, storage| router.bank.init_balance(storage, &bonder, funds).unwrap());

    let token = create_token(
        &mut router,
        &bonder,
        "fury token".to_string(),
        "FURY".to_string(),
        Uint128::new(500000),
    );

    let amm = create_amm(&mut router, &owner, &token, NATIVE_TOKEN_DENOM);

    // Add initial liquidity to pools
    // Simulate increase allowance
    let add_liquidity_msg = fanfuryswap::msg::ExecuteMsg::AddLiquidity {
        token1_amount: Uint128::new(100000),
        min_liquidity: Uint128::new(100000),
        max_token2: Uint128::new(100000),
        fee_amount: Uint128::new(2600),
        expiration: None,
    };
    let _res = router
        .execute_contract(
            bonder.clone(),
            amm.clone(),
            &add_liquidity_msg,
            &[Coin {
                denom: NATIVE_TOKEN_DENOM.into(),
                amount: Uint128::new(102600),
            }],
        )
        .unwrap();

    // Ensure balances updated
    let token1_balance = bank_balance(&mut router, &bonder, NATIVE_TOKEN_DENOM);
    assert_eq!(token1_balance, Uint128::new(400000));

    // Check pool contract info
    let res: FanfuryConfigResponse = get_pool_config(&router, &amm);

    // Create native bonding
    let bonding_contract =
        create_native_bonding(&mut router, &owner, &amm, &token, NATIVE_TOKEN_DENOM);

    let bond_msg = ExecuteMsg::Bond {
        amount: Uint128::new(10000),
    };
    let _res = router
        .execute_contract(
            bonder.clone(),
            bonding_contract.clone(),
            &bond_msg,
            &[Coin {
                denom: NATIVE_TOKEN_DENOM.into(),
                amount: Uint128::new(10130),
            }],
        )
        .unwrap();

    // Check bonding record
    let record = get_bonding_info(&router, &bonding_contract, &bonder);
    assert_eq!(record.list[0].amount, Uint128::new(9129));
}
