use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct GlobalConfig {
    pub admin: Pubkey,
    pub treasury_fee_percentage: u16,
    pub usdc_mint: Pubkey,
    pub treasury_token_account: Pubkey,
    // Add other fields as necessary, e.g., for VRF config
}

impl GlobalConfig {
    // discriminator (8) + admin (32) + fee (2) + mint (32) + treasury_token_account (32)
    pub const ACCOUNT_SIZE: usize = 8 + 32 + 2 + 32 + 32;
}
