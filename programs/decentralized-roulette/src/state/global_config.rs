use anchor_lang::prelude::*;

#[account]
pub struct GlobalConfig {
    pub authority: Pubkey,
    pub usdc_mint: Pubkey,
    pub treasury_token_account: Pubkey,
    pub treasury_usdc_account: Pubkey,    // Treasury USDC account for fees
    pub treasury_fee_percentage: u16,     // Set to 250 for 2.5%
    pub is_paused: bool,
    
    // Game configuration for autonomous games
    pub min_game_duration: i64,
    pub max_game_duration: i64,
    pub default_game_duration: i64,       // Default duration for autonomous games (300s = 5min)
    pub default_betting_duration: i64,    // Default betting duration (180s = 3min)
    pub min_bet_amount: u64,
    pub max_bet_amount: u64,
    pub max_players_per_game: u64,
    
    // Autonomous system tracking
    pub current_game_nonce: u64,          // Current nonce for next game creation
    pub total_roulettes_created: u64,     // Total games created autonomously
    pub keeper_reward_amount: u64,        // USDC reward per keeper action (micro-USDC)
    pub min_keeper_balance: u64,          // Minimum balance required to be a keeper
    
    // Metadata
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
}

impl GlobalConfig {
    pub const ACCOUNT_SIZE: usize = 8 + // discriminator
        32 + // authority
        32 + // usdc_mint  
        32 + // treasury_token_account
        32 + // treasury_usdc_account
        2 +  // treasury_fee_percentage
        1 +  // is_paused
        8 +  // min_game_duration
        8 +  // max_game_duration  
        8 +  // default_game_duration
        8 +  // default_betting_duration
        8 +  // min_bet_amount
        8 +  // max_bet_amount
        8 +  // max_players_per_game
        8 +  // current_game_nonce
        8 +  // total_roulettes_created
        8 +  // keeper_reward_amount
        8 +  // min_keeper_balance
        8 +  // created_at
        8 +  // updated_at
        1;   // bump
}