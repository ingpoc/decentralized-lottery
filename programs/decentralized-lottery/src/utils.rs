// src/utils.rs
use anchor_lang::prelude::*;
// ADD this import:
use crate::errors::LotteryError;

pub fn safe_add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(LotteryError::SafeMathError.into())
}

pub fn safe_sub(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b).ok_or(LotteryError::SafeMathError.into())
}

pub fn safe_mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(LotteryError::SafeMathError.into())
}

pub fn safe_div(a: u64, b: u64) -> Result<u64> {
    a.checked_div(b).ok_or(LotteryError::SafeMathError.into())
}

// Placeholder function to derive ticket PDA pubkey
pub fn get_ticket_pda_pubkey(lottery_key: &Pubkey, ticket_id: u64) -> Result<Pubkey> {
    // Implement actual PDA derivation using seeds [b"ticket", lottery_key.as_ref(), &ticket_id.to_le_bytes()]
    let seeds = &[
        b"ticket",
        lottery_key.as_ref(),
        &ticket_id.to_le_bytes()
    ];
    let (pda, _bump) = Pubkey::find_program_address(seeds, &crate::ID);
    // For now, just return the derived PDA, error handling could be added
    Ok(pda)
    // Err(LotteryError::PDADerivationError.into()) // Previous placeholder
}

// Safe multiplication followed by division, using u128 for intermediate values
pub fn safe_mul_div(a: u64, b: u64, divisor: u64) -> Result<u64> {
    if divisor == 0 {
        return Err(LotteryError::SafeMathError.into()); // Or specific division by zero error
    }
    (a as u128)
        .checked_mul(b as u128)
        .ok_or(LotteryError::SafeMathError)?
        .checked_div(divisor as u128)
        .ok_or(LotteryError::SafeMathError)?
        .try_into() // Convert back to u64
        .map_err(|_| LotteryError::SafeMathError.into()) // Error if conversion fails
}