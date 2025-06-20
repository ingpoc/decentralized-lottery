// src/state/treasury.rs
use anchor_lang::prelude::*;

#[account]
pub struct Treasury {
    pub multisig: Pubkey, // Multisig account for withdrawals
    pub time_lock_seconds: i64, // Time lock for treasury withdrawals
    pub last_withdrawal_time: i64,
    pub treasury_balance: u64,
}

impl Treasury {
    pub const ACCOUNT_SIZE: usize = 8 + 32 + 8 + 8 + 8; // discriminator + multisig + time_lock_seconds + last_withdrawal_time + treasury_balance
}