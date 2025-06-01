use anchor_lang::prelude::*;
use crate::state::GlobalConfig;
use crate::errors::LotteryError;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = GlobalConfig::ACCOUNT_SIZE,
        seeds = [b"global_config"],
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

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    global_config.admin = ctx.accounts.admin.key();
    global_config.treasury_fee_percentage = 250; // Default 2.5%
    global_config.usdc_mint = ctx.accounts.usdc_mint.key();
    global_config.treasury_token_account = ctx.accounts.treasury_token_account.key();
    Ok(())
}
