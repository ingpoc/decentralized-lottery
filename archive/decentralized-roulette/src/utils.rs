use anchor_lang::prelude::*;

/// Utility functions for roulette operations

pub fn calculate_winning_number(randomness: &[u8; 32], max_number: u8) -> u8 {
    // Convert first 8 bytes to u64
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&randomness[..8]);
    let random_value = u64::from_le_bytes(bytes);
    
    // Return number in range 0..max_number
    (random_value % max_number as u64) as u8
}

pub fn validate_roulette_number(number: u8, is_american: bool) -> bool {
    if is_american {
        number <= 37 // 0-36 + 00 (37)
    } else {
        number <= 36 // 0-36
    }
}

pub fn get_red_numbers() -> Vec<u8> {
    vec![1, 3, 5, 7, 9, 12, 14, 16, 18, 19, 21, 23, 25, 27, 30, 32, 34, 36]
}

pub fn get_black_numbers() -> Vec<u8> {
    vec![2, 4, 6, 8, 10, 11, 13, 15, 17, 20, 22, 24, 26, 28, 29, 31, 33, 35]
}

pub fn is_red_number(number: u8) -> bool {
    get_red_numbers().contains(&number)
}

pub fn is_black_number(number: u8) -> bool {
    get_black_numbers().contains(&number)
}

/// Derive the bet PDA for a given roulette and bet ID
pub fn get_bet_pda_pubkey(roulette: &Pubkey, bet_id: u64) -> Result<Pubkey> {
    let (pda, _) = Pubkey::find_program_address(
        &[
            crate::constants::BET_SEED,
            roulette.as_ref(),
            &bet_id.to_le_bytes(),
        ],
        &crate::id(),
    );
    Ok(pda)
}

/// Derive the roulette PDA for a given authority and nonce
pub fn get_roulette_pda_pubkey(authority: &Pubkey, nonce: u64) -> Result<Pubkey> {
    let (pda, _) = Pubkey::find_program_address(
        &[
            crate::constants::ROULETTE_SEED,
            authority.as_ref(),
            &nonce.to_le_bytes(),
        ],
        &crate::id(),
    );
    Ok(pda)
}

/// Derive the VRF client PDA for a given roulette
pub fn get_vrf_client_pda_pubkey(roulette: &Pubkey) -> Result<Pubkey> {
    let (pda, _) = Pubkey::find_program_address(
        &[
            crate::constants::VRF_CLIENT_SEED,
            roulette.as_ref(),
        ],
        &crate::id(),
    );
    Ok(pda)
}

/// Calculate total refunds needed for a roulette game
/// Returns the total amount that needs to be refunded to all bettors
pub fn calculate_total_refunds(roulette: &crate::state::roulette::RouletteAccount) -> Result<u64> {
    // For expired/cancelled games, all bets need to be refunded
    // This is a simplified calculation - in practice, you'd need to iterate through all bet accounts
    // For now, we'll use the total bet amount as a proxy
    Ok(roulette.total_bet_amount)
}

/// Validate that a given pubkey matches the expected bet PDA
pub fn validate_bet_pda(roulette: &Pubkey, bet_id: u64, bet_pda: &Pubkey) -> Result<()> {
    let expected_pda = get_bet_pda_pubkey(roulette, bet_id)?;
    require!(expected_pda == *bet_pda, crate::errors::RouletteError::InvalidAccount);
    Ok(())
}

/// Validate that a given pubkey matches the expected roulette PDA
pub fn validate_roulette_pda(authority: &Pubkey, nonce: u64, roulette_pda: &Pubkey) -> Result<()> {
    let expected_pda = get_roulette_pda_pubkey(authority, nonce)?;
    require!(expected_pda == *roulette_pda, crate::errors::RouletteError::InvalidAccount);
    Ok(())
}