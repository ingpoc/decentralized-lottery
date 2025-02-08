// src/state/treasury.rs
use anchor_lang::prelude::*;

#[account]
pub struct GlobalConfig {
    pub treasury: Pubkey,
    pub treasury_fee_percentage: u16, // Basis points (2.5% = 250)
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
}

#[account]
pub struct Treasury {
    pub multisig: Pubkey, // Multisig account for withdrawals
    pub time_lock_seconds: i64, // Time lock for treasury withdrawals
    pub last_withdrawal_time: i64,
    pub treasury_balance: u64,
}