use anchor_lang::prelude::*;
use crate::state::GlobalConfig;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = GlobalConfig::ACCOUNT_SIZE,
        seeds = [b"global_config_v2"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: This is the USDC mint account
    pub usdc_mint: AccountInfo<'info>,
    /// CHECK: This is the treasury token account
    #[account(mut)]
    pub treasury_token_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_handler(ctx: Context<Initialize>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let clock = Clock::get()?;

    global_config.admin = ctx.accounts.admin.key();
    global_config.treasury_fee_percentage = 250; // Default 2.5%
    global_config.usdc_mint = ctx.accounts.usdc_mint.key();
    global_config.treasury_token_account = ctx.accounts.treasury_token_account.key();
    global_config.is_paused = false;

    // Validation limits (sensible defaults for USDC — 6 decimals)
    global_config.min_ticket_price = 1_000_000;      // 1 USDC
    global_config.max_ticket_price = 1_000_000_000;  // 1,000 USDC
    global_config.min_draw_duration = 60;            // 1 minute
    global_config.max_draw_duration = 30 * 24 * 60 * 60; // 30 days
    global_config.max_tickets_per_lottery = 100_000;

    global_config.created_at = clock.unix_timestamp;
    global_config.updated_at = clock.unix_timestamp;
    global_config.bump = ctx.bumps.global_config;

    Ok(())
}
