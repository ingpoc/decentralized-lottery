use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::treasury::GlobalConfig;
use crate::errors::LotteryError;

#[derive(Accounts)]
pub struct CancelLottery<'info> {
    #[account(
        mut,
        seeds = [
            b"lottery",
            lottery_account.lottery_type.to_string().as_bytes(),
            &lottery_account.draw_time.to_le_bytes()
        ],
        bump
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        seeds = [b"global_config"],
        bump,
        constraint = global_config.admin == admin.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub admin: Signer<'info>,
}

pub fn handler(ctx: Context<CancelLottery>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    
    // Validate if lottery can be cancelled
    if !lottery_account.state.can_cancel() {
        return Err(LotteryError::InvalidCancellation.into());
    }

    // Set state to cancelled
    lottery_account.state = LotteryState::Cancelled;

    // Note: Refund mechanism will be implemented separately
    // This just marks the lottery as cancelled, allowing refunds to be processed

    Ok(())
} 