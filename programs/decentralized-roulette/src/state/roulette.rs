use anchor_lang::prelude::*;
use crate::errors::RouletteError;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RouletteType {
    European,  // 37 numbers (0-36)
    American,  // 38 numbers (0-36 + 00)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum RouletteState {
    Created,
    Open,           // Accepting bets (3 minutes)
    Locked,         // Bets closed, ready to spin (30 seconds)
    Spinning,       // VRF request in progress (30 seconds)
    AwaitingRandomness, // Waiting for VRF result
    Completed,      // Results determined, payouts available
    Expired,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum BetType {
    Straight,       // Single number (35:1) - 1 number
    Split,          // Two adjacent numbers (17:1) - 2 numbers
    Street,         // Three numbers in a row (11:1) - 3 numbers
    Corner,         // Four numbers in a square (8:1) - 4 numbers
    SixLine,        // Six numbers in two rows (5:1) - 6 numbers
    Red,            // All red numbers (1:1) - 18 numbers
    Black,          // All black numbers (1:1) - 18 numbers
    Even,           // All even numbers (1:1) - 18 numbers
    Odd,            // All odd numbers (1:1) - 18 numbers
    Low,            // 1-18 (1:1) - 18 numbers
    High,           // 19-36 (1:1) - 18 numbers
    FirstTwelve,    // 1-12 (2:1) - 12 numbers
    SecondTwelve,   // 13-24 (2:1) - 12 numbers
    ThirdTwelve,    // 25-36 (2:1) - 12 numbers
    FirstColumn,    // First column (2:1) - 12 numbers
    SecondColumn,   // Second column (2:1) - 12 numbers
    ThirdColumn,    // Third column (2:1) - 12 numbers
}

#[account]
pub struct RouletteAccount {
    pub roulette_type: RouletteType,
    pub min_bet: u64,
    pub max_bet: u64,
    pub game_duration: i64,      // 5 minutes (300 seconds)
    pub betting_duration: i64,   // 3 minutes (180 seconds)
    pub start_time: i64,
    pub betting_end_time: i64,   // start_time + 180 seconds
    pub spin_time: i64,          // start_time + 210 seconds
    pub reveal_time: i64,        // start_time + 240 seconds
    pub end_time: i64,           // start_time + 300 seconds
    
    // Game state
    pub state: RouletteState,
    pub total_bets: u64,
    pub total_bet_amount: u64,
    pub total_players: u64,
    pub winning_number: Option<u8>,
    pub last_bet_id: u64,
    
    // Authority and config
    pub created_by: Pubkey,
    pub authority: Pubkey,
    pub global_config: Pubkey,
    pub roulette_usdc_account: Pubkey,
    
    // VRF-related fields (same pattern as lottery)
    pub vrf_client: Option<Pubkey>,
    pub vrf_randomness: Option<[u8; 32]>,
    pub vrf_request_key: Option<Pubkey>,
    pub randomness_fulfilled: bool,
    // Switchboard VRF fields
    pub switchboard_vrf: Option<Pubkey>,
    pub vrf_request_timestamp: Option<i64>,
    
    // Financial tracking
    pub total_payouts: u64,
    pub house_edge_collected: u64,
    pub treasury_fee_collected: u64,
    pub is_settled: bool,
    
    // Metadata
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub nonce: u64,
    pub bump: u8,
}

impl RouletteAccount {
    pub const ACCOUNT_SIZE: usize = 8 + // Discriminator
        1 +            // roulette_type enum (1 byte for simple enum)
        8 +            // min_bet u64
        8 +            // max_bet u64
        8 +            // game_duration i64
        8 +            // betting_duration i64
        8 +            // start_time i64
        8 +            // betting_end_time i64
        8 +            // spin_time i64
        8 +            // reveal_time i64
        8 +            // end_time i64
        
        // Game state
        1 +            // state enum
        8 +            // total_bets u64
        8 +            // total_bet_amount u64
        8 +            // total_players u64
        (1 + 1) +      // Option<u8> winning_number
        8 +            // last_bet_id u64
        
        // Authority
        32 +           // created_by Pubkey
        32 +           // authority Pubkey
        32 +           // global_config Pubkey
        32 +           // roulette_usdc_account Pubkey
        
        // VRF fields
        (1 + 32) +     // Option<Pubkey> vrf_client
        (1 + 32) +     // Option<[u8; 32]> vrf_randomness
        (1 + 32) +     // Option<Pubkey> vrf_request_key
        1 +            // randomness_fulfilled bool
        // Switchboard VRF fields
        (1 + 32) +     // Option<Pubkey> switchboard_vrf
        (1 + 8) +      // Option<i64> vrf_request_timestamp
        
        // Financial tracking
        8 +            // total_payouts u64
        8 +            // house_edge_collected u64
        8 +            // treasury_fee_collected u64
        1 +            // is_settled bool
        
        // Metadata
        8 +            // created_at i64
        (1 + 8) +      // Option<i64> completed_at
        8 +            // nonce u64
        1;             // bump u8
    
    pub fn calculate_payout(&self, bet_type: &BetType, bet_amount: u64) -> Result<u64> {
        let multiplier = match bet_type {
            BetType::Straight => 35,        // 35:1
            BetType::Split => 17,           // 17:1
            BetType::Street => 11,          // 11:1
            BetType::Corner => 8,           // 8:1
            BetType::SixLine => 5,          // 5:1
            BetType::Red | BetType::Black | 
            BetType::Even | BetType::Odd | 
            BetType::Low | BetType::High => 1, // 1:1
            BetType::FirstTwelve | BetType::SecondTwelve | 
            BetType::ThirdTwelve | BetType::FirstColumn |
            BetType::SecondColumn | BetType::ThirdColumn => 2, // 2:1
        };
        
        // Safe arithmetic with overflow protection
        bet_amount
            .checked_mul(multiplier + 1)
            .ok_or(RouletteError::PayoutOverflow.into())
    }
    
    pub fn is_betting_open(&self, current_time: i64) -> bool {
        self.state == RouletteState::Open && current_time < self.betting_end_time
    }
    
    pub fn can_spin(&self, current_time: i64) -> bool {
        self.state == RouletteState::Locked && current_time >= self.spin_time
    }
    
    pub fn can_reveal(&self, current_time: i64) -> bool {
        self.state == RouletteState::AwaitingRandomness && current_time >= self.reveal_time
    }
    
    pub fn get_numbers_for_bet_type(&self, bet_type: &BetType, bet_numbers: &[u8]) -> Vec<u8> {
        match bet_type {
            BetType::Straight => bet_numbers.to_vec(),
            BetType::Split => bet_numbers.to_vec(),
            BetType::Street => bet_numbers.to_vec(),
            BetType::Corner => bet_numbers.to_vec(),
            BetType::SixLine => bet_numbers.to_vec(),
            BetType::Red => vec![1, 3, 5, 7, 9, 12, 14, 16, 18, 19, 21, 23, 25, 27, 30, 32, 34, 36],
            BetType::Black => vec![2, 4, 6, 8, 10, 11, 13, 15, 17, 20, 22, 24, 26, 28, 29, 31, 33, 35],
            BetType::Even => (2..=36).step_by(2).collect(),
            BetType::Odd => (1..=35).step_by(2).collect(),
            BetType::Low => (1..=18).collect(),
            BetType::High => (19..=36).collect(),
            BetType::FirstTwelve => (1..=12).collect(),
            BetType::SecondTwelve => (13..=24).collect(),
            BetType::ThirdTwelve => (25..=36).collect(),
            BetType::FirstColumn => vec![1, 4, 7, 10, 13, 16, 19, 22, 25, 28, 31, 34],
            BetType::SecondColumn => vec![2, 5, 8, 11, 14, 17, 20, 23, 26, 29, 32, 35],
            BetType::ThirdColumn => vec![3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36],
        }
    }
    
    pub fn is_winning_bet(&self, bet_type: &BetType, bet_numbers: &[u8], winning_number: u8) -> bool {
        let valid_numbers = self.get_numbers_for_bet_type(bet_type, bet_numbers);
        valid_numbers.contains(&winning_number)
    }
}

impl RouletteState {
    pub fn can_transition_to(&self, next_state: &RouletteState) -> bool {
        match self {
            RouletteState::Created => matches!(next_state, RouletteState::Open | RouletteState::Cancelled),
            RouletteState::Open => matches!(next_state, RouletteState::Locked | RouletteState::Cancelled | RouletteState::Expired),
            RouletteState::Locked => matches!(next_state, RouletteState::Spinning | RouletteState::Cancelled | RouletteState::Expired),
            RouletteState::Spinning => matches!(next_state, RouletteState::AwaitingRandomness | RouletteState::Cancelled),
            RouletteState::AwaitingRandomness => matches!(next_state, RouletteState::Completed | RouletteState::Cancelled),
            RouletteState::Completed => false, // Terminal state
            RouletteState::Expired => false,   // Terminal state
            RouletteState::Cancelled => false, // Terminal state
        }
    }
}