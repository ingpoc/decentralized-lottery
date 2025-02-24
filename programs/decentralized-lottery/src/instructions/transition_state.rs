use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::treasury::GlobalConfig;
use crate::errors::LotteryError;

#[derive(Accounts)]
pub struct TransitionState<'info> {
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

pub fn handler(ctx: Context<TransitionState>, next_state: LotteryState) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let current_state = &lottery_account.state;

    // Validate state transition
    if !current_state.can_transition_to(&next_state) {
        return Err(LotteryError::InvalidStateTransition.into());
    }

    // Additional validations based on specific transitions
    match (&lottery_account.state, &next_state) {
        (LotteryState::Created, LotteryState::Open) => {
            // Validate draw time hasn't passed
            if lottery_account.draw_time <= Clock::get()?.unix_timestamp {
                return Err(LotteryError::InvalidDrawTime.into());
            }
        },
        (LotteryState::Open, LotteryState::Drawing) => {
            // Validate draw time has been reached
            if lottery_account.draw_time > Clock::get()?.unix_timestamp {
                return Err(LotteryError::InvalidDrawTime.into());
            }
        },
        _ => {}
    }

    // Update state
    lottery_account.state = next_state;

    Ok(())
} 