use crate::error::ContractError;
use crate::msg::{
    AllBondStateResponse, BondStateResponse, BondingRecord, ConfigResponse, ExecuteMsg,
    InstantiateMsg, MigrateMsg, QueryMsg,
};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    QueryRequest, Response, StdResult, Storage, Uint128, WasmQuery, BankMsg, Coin,
};
use cw2::{get_contract_version, set_contract_version};
use cw_storage_plus::Bound;
use cw_utils::maybe_addr;

use crate::state::{Config, BONDING, CONFIG, FEE_WALLET};
use crate::util::{NORMAL_DECIMAL, THOUSAND};
use wasmswap::msg::{
    QueryMsg as WasmswapQueryMsg, Token1ForToken2PriceResponse, Token2ForToken1PriceResponse,
};

// Version info, for migration info
const CONTRACT_NAME: &str = "fanfurybonding";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let config = Config {
        owner: msg.owner.clone(),
        pool_address: msg.pool_address.clone(),
        treasury_address: msg.treasury_address.clone(),
        fury_token_denom: msg.fury_token_denom.clone(),
        lock_seconds: msg.lock_seconds,
        discount: msg.discount,
        usdc_denom: msg.usdc_denom.clone(),
        is_native_bonding: msg.is_native_bonding,
        tx_fee: msg.tx_fee,
        platform_fee: msg.platform_fee,
        enabled: true,
        daily_vesting_amount: msg.daily_vesting_amount,
        cumulated_amount: Uint128::zero(),
        daily_current_bond_amount: Uint128::zero(),
        last_timestamp: env.block.time.seconds(),
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateOwner { owner } => execute_update_owner(deps, env, info, owner),
        ExecuteMsg::UpdateEnabled { enabled } => execute_update_enabled(deps, env, info, enabled),
        ExecuteMsg::UpdateCoinDenom { denom } => execute_update_coin_denom(deps, env, info, denom),
        ExecuteMsg::UpdateConfig {
            treasury_address,
            lock_seconds,
            discount,
            tx_fee,
            platform_fee,
            daily_vesting_amount,
        } => execute_update_config(
            deps,
            env,
            info,
            treasury_address,
            lock_seconds,
            discount,
            tx_fee,
            platform_fee,
            daily_vesting_amount,
        ),
        ExecuteMsg::Bond { amount } => execute_bond(deps, env, info, amount),
        ExecuteMsg::LpBond { address, amount } => execute_lp_bond(deps, env, info, address, amount),
        ExecuteMsg::Unbond {} => execute_unbond(deps, env, info),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, env, info, amount),
        ExecuteMsg::ChangeFeeWallet { address } => change_fee_wallet(deps, info, address),
    }
}

pub fn check_enabled(storage: &dyn Storage) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(storage)?;
    if !cfg.enabled {
        return Err(ContractError::Disabled {});
    }
    Ok(Response::new().add_attribute("action", "check_enabled"))
}

pub fn check_owner(storage: &dyn Storage, address: Addr) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(storage)?;
    if cfg.owner != address {
        return Err(ContractError::Unauthorized {});
    }
    Ok(Response::new().add_attribute("action", "check_owner"))
}

pub fn change_fee_wallet(
    deps: DepsMut,
    info: MessageInfo,
    address: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    deps.api.addr_validate(&address)?;

    FEE_WALLET.save(deps.storage, &address)?;
    Ok(Response::new()
        .add_attribute("action", "change_fee_wallet")
        .add_attribute("fee_wallet", address))
}

pub fn execute_update_owner(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: Addr,
) -> Result<Response, ContractError> {
    check_owner(deps.storage, info.sender)?;

    let mut cfg = CONFIG.load(deps.storage)?;

    cfg.owner = owner.clone();

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_owner"), attr("owner", owner)]))
}

pub fn execute_update_coin_denom(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    denom: String, // Adjust the type as per your actual field type
) -> Result<Response, ContractError> {
    check_owner(deps.storage, info.sender)?;

    let mut cfg = CONFIG.load(deps.storage)?;

    cfg.usdc_denom = denom.clone(); // Update based on your specific field

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attributes(vec![attr("action", "update_denom"), attr("denom", denom)]))
}

pub fn execute_update_enabled(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    enabled: bool,
) -> Result<Response, ContractError> {
    check_owner(deps.storage, info.sender)?;

    let mut cfg = CONFIG.load(deps.storage)?;

    cfg.enabled = enabled;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_enabled"),
        attr("enabled", enabled.to_string()),
    ]))
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    treasury_address: Addr,
    lock_seconds: u64,
    discount: u64,
    tx_fee: u64,
    platform_fee: u64,
    daily_vesting_amount: Uint128,
) -> Result<Response, ContractError> {
    check_owner(deps.storage, info.sender)?;

    let mut cfg = CONFIG.load(deps.storage)?;

    cfg.treasury_address = treasury_address.clone();
    cfg.lock_seconds = lock_seconds;
    cfg.discount = discount;
    cfg.tx_fee = tx_fee;
    cfg.platform_fee = platform_fee;
    cfg.daily_vesting_amount = daily_vesting_amount;

    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "update_config"),
        attr("treasury_address", treasury_address.to_string()),
        attr("lock_seconds", lock_seconds.to_string()),
        attr("discount", discount.to_string()),
        attr("tx_fee", tx_fee.to_string()),
        attr("platform_fee", platform_fee.to_string()),
        attr("daily_vesting_amount", daily_vesting_amount.to_string()),
    ]))
}

fn check_daily_vesting_amount(storage: &mut dyn Storage, current_time: u64, receiving_amount: Uint128) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(storage)?;
    let last_timestamp = cfg.last_timestamp;
    let current_bond_amount = cfg.daily_current_bond_amount;

    if last_timestamp + THOUSAND < current_time {
        cfg.last_timestamp = current_time;
        cfg.daily_current_bond_amount = receiving_amount;
    } else {
        if current_bond_amount + receiving_amount > cfg.daily_vesting_amount {
            return Err(ContractError::InsufficientDailyVestingAmount {});
        }
        cfg.daily_current_bond_amount += receiving_amount;
    }

    CONFIG.save(storage, &cfg)?;
    Ok(Response::new())
}

fn subtract_daily_vesting_amount(storage: &mut dyn Storage, receiving_amount: Uint128) -> Result<Response, ContractError> {
    let mut cfg = CONFIG.load(storage)?;
    cfg.cumulated_amount += receiving_amount;
    if cfg.cumulated_amount >= receiving_amount {
        cfg.cumulated_amount -= receiving_amount;
    } else {
        cfg.cumulated_amount = Uint128::zero();
    }
    CONFIG.save(storage, &cfg)?;
    Ok(Response::new())
}


pub fn execute_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    check_enabled(deps.storage)?;

    let cfg = CONFIG.load(deps.storage)?;

    let usdc_coin = info
        .funds
        .iter()
        .find(|&coin| coin.denom == cfg.usdc_denom)
        .ok_or(ContractError::InsufficientFury {})?;

    if usdc_coin.amount < amount {
        return Err(ContractError::InsufficientFury {});
    }

    check_daily_vesting_amount(deps.storage, env.block.time.seconds(), amount)?;

    let price_response: Token2ForToken1PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.pool_address.to_string(),
        msg: to_binary(&WasmswapQueryMsg::Token2ForToken1Price { token2_amount: amount })?,
    }))?;

    let bonding_amount = price_response.token1_amount
        * Uint128::from((NORMAL_DECIMAL - cfg.discount) as u64)
        / Uint128::from(NORMAL_DECIMAL as u64);

    let mut current_bond = BONDING.load(deps.storage, info.sender.clone()).unwrap_or_default();
    current_bond.owner = info.sender.clone();
    current_bond.bond_amount += bonding_amount;
    current_bond.bond_timestamp = env.block.time.seconds();

    BONDING.save(deps.storage, info.sender.clone(), &current_bond)?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "bond"),
        attr("address", info.sender.to_string()),
        attr("amount", amount.to_string()),
        attr("bonding_amount", bonding_amount.to_string()),
    ]))
}

pub fn execute_lp_bond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    address: Addr,
    amount: Uint128,
) -> Result<Response, ContractError> {
    check_enabled(deps.storage)?;

    check_daily_vesting_amount(deps.storage, env.block.time.seconds(), amount)?;

    let cfg = CONFIG.load(deps.storage)?;

    let price_response: Token1ForToken2PriceResponse = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: cfg.pool_address.to_string(),
        msg: to_binary(&WasmswapQueryMsg::Token1ForToken2Price { token1_amount: amount })?,
    }))?;

    let bonding_amount = price_response.token2_amount
        * Uint128::from((NORMAL_DECIMAL - cfg.discount) as u64)
        / Uint128::from(NORMAL_DECIMAL as u64);

    let mut current_bond = BONDING.load(deps.storage, address.clone()).unwrap_or_default();
    current_bond.owner = address.clone();
    current_bond.bond_amount += bonding_amount;
    current_bond.bond_timestamp = env.block.time.seconds();

    BONDING.save(deps.storage, address.clone(), &current_bond)?;

    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: cfg.treasury_address.to_string(),
        amount: vec![Coin { denom: cfg.usdc_denom.clone(), amount }],
    });

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attributes(vec![
            attr("action", "lp_bond"),
            attr("address", address.to_string()),
            attr("amount", amount.to_string()),
            attr("bonding_amount", bonding_amount.to_string()),
        ]))
}

pub fn execute_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    check_enabled(deps.storage)?;
    let cfg = CONFIG.load(deps.storage)?;

    let current_bond = BONDING.load(deps.storage, info.sender.clone())?;

    if env.block.time.seconds() < current_bond.bond_timestamp + cfg.lock_seconds {
        return Err(ContractError::UnbondingLock {});
    }

    BONDING.remove(deps.storage, info.sender.clone());

    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin { denom: cfg.fury_token_denom.clone(), amount: current_bond.bond_amount }],
    });

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attributes(vec![
            attr("action", "unbond"),
            attr("address", info.sender.to_string()),
            attr("amount", current_bond.bond_amount.to_string()),
        ]))
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    check_owner(deps.storage, info.sender)?;

    let cfg = CONFIG.load(deps.storage)?;

    let usdc_balance = deps.querier.query_balance(env.contract.address.clone(), cfg.usdc_denom.clone())?;

    if usdc_balance.amount < amount {
        return Err(ContractError::InsufficientFury {});
    }

    let bank_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin { denom: cfg.usdc_denom.clone(), amount }],
    });

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attributes(vec![
            attr("action", "withdraw"),
            attr("address", info.sender.to_string()),
            attr("amount", amount.to_string()),
        ]))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => query_config(deps, env),
        QueryMsg::BondState { address } => query_bond_state(deps, env, address),
        QueryMsg::AllBondState { start_after, limit } => query_all_bond_state(deps, env, start_after, limit),
        QueryMsg::GetFeeWallet {} => query_fee_wallet(deps),
    }
}

pub fn query_config(deps: Deps, _env: Env) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    to_binary(&ConfigResponse {
        owner: config.owner,
        pool_address: config.pool_address,
        treasury_address: config.treasury_address,
        fury_token_denom: config.fury_token_denom,
        lock_seconds: config.lock_seconds,
        discount: config.discount,
        usdc_denom: config.usdc_denom,
        is_native_bonding: config.is_native_bonding,
        tx_fee: config.tx_fee,
        platform_fee: config.platform_fee,
        enabled: config.enabled,
        daily_vesting_amount: config.daily_vesting_amount,
        cumulated_amount: config.cumulated_amount,
        daily_current_bond_amount: config.daily_current_bond_amount,
        last_timestamp: config.last_timestamp,
    })
}

pub fn query_bond_state(deps: Deps, _env: Env, address: Addr) -> StdResult<Binary> {
    let bond_state = BONDING.load(deps.storage, address.clone())?;
    to_binary(&BondStateResponse {
        address,
        list: vec![BondingRecord {
            bond_amount: bond_state.bond_amount,
            bond_timestamp: bond_state.bond_timestamp,
        }],
        unbond_amount: bond_state.bond_amount,
        fee_amount: Uint128::zero(),
    })
}

pub fn query_all_bond_state(
    deps: Deps,
    _env: Env,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let limit = limit.unwrap_or(10).min(30) as usize;
    let start = start_after.map(|s| Bound::ExclusiveRaw(s.into_bytes()));

    let all_bond_state: StdResult<Vec<_>> = BONDING
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (addr, bond_state) = item?;
            Ok(BondStateResponse {
                address: deps.api.addr_validate(&addr)?,
                list: vec![BondingRecord {
                    bond_amount: bond_state.bond_amount,
                    bond_timestamp: bond_state.bond_timestamp,
                }],
                unbond_amount: bond_state.bond_amount, // Correct this to reflect actual logic
                fee_amount: Uint128::zero(), // Correct this to reflect actual logic
            })
        })
        .collect();

    to_binary(&AllBondStateResponse {
        list: all_bond_state?,
    })
}


pub fn query_fee_wallet(deps: Deps) -> StdResult<Binary> {
    let fee_wallet = FEE_WALLET.load(deps.storage)?;
    to_binary(&fee_wallet)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}
