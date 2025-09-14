// src/utils.rs
use anchor_lang::prelude::*;
use anchor_lang::solana_program::pubkey::Pubkey;

/// Derive the ticket PDA for a given lottery and ticket ID
pub fn get_ticket_pda_pubkey(lottery: &Pubkey, ticket_id: u64) -> Result<Pubkey> {
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"ticket",
            lottery.as_ref(),
            &ticket_id.to_le_bytes(),
        ],
        &crate::id(),
    );
    Ok(pda)
}

/// Derive the lottery PDA for a given authority and nonce
pub fn get_lottery_pda_pubkey(authority: &Pubkey, nonce: u64) -> Result<Pubkey> {
    let (pda, _) = Pubkey::find_program_address(
        &[
            b"lottery",
            authority.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &crate::id(),
    );
    Ok(pda)
}

/// Validate that a given pubkey matches the expected ticket PDA
pub fn validate_ticket_pda(lottery: &Pubkey, ticket_id: u64, ticket_pda: &Pubkey) -> Result<()> {
    let expected_pda = get_ticket_pda_pubkey(lottery, ticket_id)?;
    require!(expected_pda == *ticket_pda, crate::errors::LotteryError::InvalidAccount);
    Ok(())
}

/// Validate that a given pubkey matches the expected lottery PDA
pub fn validate_lottery_pda(authority: &Pubkey, nonce: u64, lottery_pda: &Pubkey) -> Result<()> {
    let expected_pda = get_lottery_pda_pubkey(authority, nonce)?;
    require!(expected_pda == *lottery_pda, crate::errors::LotteryError::InvalidAccount);
    Ok(())
}

pub fn safe_add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(crate::errors::LotteryError::SafeMathError.into())
}

pub fn safe_sub(a: u64, b: u64) -> Result<u64> {
    a.checked_sub(b).ok_or(crate::errors::LotteryError::SafeMathError.into())
}

pub fn safe_mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(crate::errors::LotteryError::SafeMathError.into())
}

pub fn safe_div(a: u64, b: u64) -> Result<u64> {
    a.checked_div(b).ok_or(crate::errors::LotteryError::SafeMathError.into())
}

// Safe multiplication followed by division, using u128 for intermediate values
pub fn safe_mul_div(a: u64, b: u64, divisor: u64) -> Result<u64> {
    if divisor == 0 {
        return Err(crate::errors::LotteryError::SafeMathError.into()); // Or specific division by zero error
    }
    (a as u128)
        .checked_mul(b as u128)
        .ok_or(crate::errors::LotteryError::SafeMathError)?
        .checked_div(divisor as u128)
        .ok_or(crate::errors::LotteryError::SafeMathError)?
        .try_into() // Convert back to u64
        .map_err(|_| crate::errors::LotteryError::SafeMathError.into()) // Error if conversion fails
}