use anchor_lang::prelude::*;
use crate::state::GlobalConfig; // Assuming global_config is in state module
use crate::errors::LotteryError;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"global_config"],
        bump, // Make sure bump is handled correctly if needed
        constraint = global_config.admin == admin.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: New USDC mint, validated by instruction logic if necessary
    pub new_usdc_mint: AccountInfo<'info>,
    /// CHECK: New treasury token account, validated by instruction logic if necessary
    pub new_treasury_token_account: AccountInfo<'info>,
}

pub fn handler(ctx: Context<UpdateConfig>, /* new_fee_percentage: Option<u16> */) -> Result<()> {
    // Placeholder logic
    let global_config = &mut ctx.accounts.global_config;
    global_config.usdc_mint = ctx.accounts.new_usdc_mint.key();
    global_config.treasury_token_account = ctx.accounts.new_treasury_token_account.key();
    // if let Some(fee) = new_fee_percentage {
    //     global_config.treasury_fee_percentage = fee;
    // }
    msg!("Placeholder: update_config instruction called");
    Ok(())
}
