use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::msg::BondingRecord;
use cosmwasm_std::{Addr, Uint128};
use cw20::Denom;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub pool_address: Addr,
    pub treasury_address: Addr,
    pub fury_token_denom: Denom,
    pub lock_seconds: u64,
    pub discount: u64,
    pub usdc_denom: Denom,
    pub is_native_bonding: bool,
    pub tx_fee: u64,
    pub platform_fee: u64,
    pub enabled: bool,
    pub daily_vesting_amount: Uint128,
    pub cumulated_amount: Uint128,
    pub daily_current_bond_amount: Uint128,
    pub last_timestamp: u64,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const BONDING_KEY: &str = "bonding";
pub const BONDING: Map<Addr, Vec<BondingRecord>> = Map::new(BONDING_KEY);
pub const FEE_WALLET: Item<String> = Item::new("config-new-fee-wallet");
