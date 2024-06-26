use std::ops::Add;

use cosmwasm_std::{Addr, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub pool_address: Addr,
    pub treasury_address: Addr,
    pub fury_token_denom: String,
    pub usdc_denom: String,
    pub lock_seconds: u64,
    pub discount: u64,
    pub tx_fee: u64,
    pub platform_fee: u64,
    pub daily_vesting_amount: Uint128,
    pub is_native_bonding: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UpdateOwner {
        owner: Addr,
    },
    UpdateEnabled {
        enabled: bool,
    },
    UpdateConfig {
        treasury_address: Addr,
        lock_seconds: u64,
        discount: u64,
        tx_fee: u64,
        platform_fee: u64,
        daily_vesting_amount: Uint128,
    },
    UpdateCoinDenom {
        denom: String,
    },
    Bond {
        amount: Uint128,
    },
    LpBond {
        address: Addr,
        amount: Uint128,
    },
    Unbond {},
    Withdraw {
        amount: Uint128,
    },
    ChangeFeeWallet {
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    BondState {
        address: Addr,
    },
    AllBondState {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    GetFeeWallet {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct ConfigResponse {
    pub owner: Addr,
    pub pool_address: Addr,
    pub treasury_address: Addr,
    pub fury_token_denom: String,
    pub lock_seconds: u64,
    pub discount: u64,
    pub usdc_denom: String,
    pub is_native_bonding: bool,
    pub tx_fee: u64,
    pub platform_fee: u64,
    pub enabled: bool,
    pub daily_vesting_amount: Uint128,
    pub cumulated_amount: Uint128,
    pub daily_current_bond_amount: Uint128,
    pub last_timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct MigrateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct BondingRecord {
    pub amount: Uint128,
    pub timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct BondStateResponse {
    pub address: Addr,
    pub list: Vec<BondingRecord>,
    pub unbond_amount: Uint128,
    pub fee_amount: Uint128,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct AllBondStateResponse {
    pub list: Vec<BondStateResponse>,
}
