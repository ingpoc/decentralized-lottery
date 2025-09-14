use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::{LotteryStateChanged, DrawingStarted};

#[derive(Accounts)]
pub struct TransitionState<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.nonce.to_le_bytes()],
        bump
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        seeds = [b"global_config_v2"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub admin: Signer<'info>, // Admin who is authorizing the transition

    pub system_program: Program<'info, System>,
}

pub fn transition_state_handler(ctx: Context<TransitionState>, next_state_param: LotteryState) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let clock = Clock::get()?;
    let current_state = lottery_account.state.clone();

    // Admin constraint for manual transitions
    if ctx.accounts.global_config.admin != ctx.accounts.admin.key() {
        return Err(LotteryError::AdminRequired.into());
    }

    if !current_state.can_transition_to(&next_state_param) {
        return Err(LotteryError::InvalidStateTransition.into());
    }

    let mut actual_next_state = next_state_param.clone();

    match (&current_state, &next_state_param) {
        (LotteryState::Created, LotteryState::Open) => {
            // Standard transition
        },
        (LotteryState::Open, LotteryState::Locked) => {
            // Lock the lottery - stop accepting new tickets
            // This prepares the lottery for drawing by preventing further ticket sales
            lottery_account.set_is_prize_pool_locked(true);
        },
        (LotteryState::Locked, LotteryState::Drawing) => {
            // Transition from Locked to Drawing - begin the drawing process
            if lottery_account.total_tickets == 0 {
                lottery_account.state = LotteryState::Expired;
                lottery_account.completed_at = Some(clock.unix_timestamp);
                emit!(LotteryStateChanged {
                    lottery_id: lottery_account.key(),
                    previous_state: current_state,
                    new_state: LotteryState::Expired,
                    timestamp: clock.unix_timestamp,
                    total_tickets_sold: lottery_account.total_tickets,
                    current_prize_pool: lottery_account.prize_pool,
                });
                return Ok(());
            }
            // Ensure draw time has passed for Locked -> Drawing
            if clock.unix_timestamp < lottery_account.draw_time {
                return Err(LotteryError::DrawTimeNotReached.into());
            }

            // Check if VRF is configured for this lottery
            if lottery_account.vrf_client.is_none() {
                msg!("WARNING: No VRF client configured, lottery will use fallback randomness");
                // Allow transition but log warning
            }

            // Transition to Drawing state - VRF request will be handled separately
            actual_next_state = LotteryState::Drawing;

            // Emit drawing started event
            emit!(DrawingStarted {
                lottery_id: lottery_account.key(),
                timestamp: clock.unix_timestamp,
                total_tickets: lottery_account.total_tickets,
                prize_pool: lottery_account.prize_pool,
                vrf_client: lottery_account.vrf_client,
            });
        },
        (LotteryState::Drawing, LotteryState::AwaitingRandomness) => {
            // This transition should only happen through VRF request, not manual
            return Err(LotteryError::InvalidStateTransition.into());
        },
        (LotteryState::AwaitingRandomness, LotteryState::Completed) => {
            // This should happen through consume_randomness or settle_randomness
            return Err(LotteryError::InvalidStateTransition.into());
        },
        _ => {
            // Handle other valid transitions
        }
    }

    // Update the lottery state
    lottery_account.state = actual_next_state.clone();

    emit!(LotteryStateChanged {
        lottery_id: lottery_account.key(),
        previous_state: current_state,
        new_state: actual_next_state,
        timestamp: clock.unix_timestamp,
        total_tickets_sold: lottery_account.total_tickets,
        current_prize_pool: lottery_account.prize_pool,
    });

    Ok(())
}
