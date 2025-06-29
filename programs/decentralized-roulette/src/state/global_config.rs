use anchor_lang::prelude::*;

#[account]
pub struct GlobalConfig {
    pub authority: Pubkey,
    pub usdc_mint: Pubkey,
    pub treasury_token_account: Pubkey,
    pub treasury_fee_percentage: u16,     // 200 = 2.00%
    pub is_paused: bool,
    pub min_game_duration: i64,           // Minimum 5 minutes
    pub max_game_duration: i64,           // Maximum duration allowed
    pub min_bet_amount: u64,              // Minimum bet in USDC (micro-lamports)
    pub max_bet_amount: u64,              // Maximum bet in USDC
    pub max_players_per_game: u64,        // Maximum number of players
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
}

impl GlobalConfig {
    pub const ACCOUNT_SIZE: usize = 8 + // Discriminator
        32 +           // authority Pubkey
        32 +           // usdc_mint Pubkey
        32 +           // treasury_token_account Pubkey
        2 +            // treasury_fee_percentage u16
        1 +            // is_paused bool
        8 +            // min_game_duration i64
        8 +            // max_game_duration i64
        8 +            // min_bet_amount u64
        8 +            // max_bet_amount u64
        8 +            // max_players_per_game u64
        8 +            // created_at i64
        8 +            // updated_at i64
        1;             // bump u8
}