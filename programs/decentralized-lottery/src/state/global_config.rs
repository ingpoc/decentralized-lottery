use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct GlobalConfig {
    pub admin: Pubkey,
    pub treasury_fee_percentage: u16,
    pub usdc_mint: Pubkey,
    pub treasury_token_account: Pubkey,
    
    // === EMERGENCY CONTROLS ===
    pub is_paused: bool,
    
    // === VALIDATION LIMITS ===
    pub min_ticket_price: u64,
    pub max_ticket_price: u64,
    pub min_draw_duration: i64,
    pub max_draw_duration: i64,
    pub max_tickets_per_lottery: u64,
    
    // === AUDIT TRAIL ===
    pub created_at: i64,
    pub updated_at: i64,
    
    // === PDA ===
    pub bump: u8,
}

impl GlobalConfig {
    // discriminator (8) + admin (32) + fee (2) + mint (32) + treasury (32) +
    // is_paused (1) + min_ticket (8) + max_ticket (8) + min_duration (8) + 
    // max_duration (8) + max_tickets (8) + created_at (8) + updated_at (8) + bump (1)
    pub const ACCOUNT_SIZE: usize = 8 + 32 + 2 + 32 + 32 + 1 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1;
}
