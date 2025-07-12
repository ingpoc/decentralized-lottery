use anchor_lang::prelude::*;

#[account]
pub struct GlobalConfig {
    pub authority: Pubkey,
    pub usdc_mint: Pubkey,
    pub treasury_token_account: Pubkey,
    pub treasury_fee_percentage: u16,     // Set to 250 for 2.5%
    pub is_paused: bool,
    pub min_game_duration: i64,
    pub max_game_duration: i64,
    pub min_bet_amount: u64,
    pub max_bet_amount: u64,
    pub max_players_per_game: u64,
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
}

impl GlobalConfig {
    pub const ACCOUNT_SIZE: usize = 8 + 32 + 32 + 32 + 2 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1;
}