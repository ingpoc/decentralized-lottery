// use anchor_lang::prelude::*;

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