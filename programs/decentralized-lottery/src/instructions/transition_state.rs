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

pub fn handler(ctx: Context<TransitionState>, next_state_param: LotteryState) -> Result<()> {
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
            lottery_account.is_prize_pool_locked = true;
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

            // PRODUCTION: Always use VRF for secure randomness
            if lottery_account.vrf_client.is_none() {
                return Err(LotteryError::VrfClientNotSet.into());
            }

            // Request VRF randomness and transition to AwaitingRandomness
            lottery_account.vrf_request_key = lottery_account.vrf_client;
            lottery_account.randomness_fulfilled = false;
            actual_next_state = LotteryState::AwaitingRandomness;

            // Emit drawing started event
            emit!(DrawingStarted {
                lottery_id: lottery_account.key(),
                timestamp: clock.unix_timestamp,
                total_tickets: lottery_account.total_tickets,
                prize_pool: lottery_account.prize_pool,
                vrf_client: lottery_account.vrf_client,
            });
        },
        (LotteryState::Open, LotteryState::Drawing) => { // This will now become AwaitingRandomness
            if lottery_account.draw_time > clock.unix_timestamp && !lottery_account.auto_transition {
                 // Manual transition to Drawing before draw_time by admin
                 // Or if auto_transition is true, this check might be bypassed or handled by cron
            }
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

            // CPI to VRF provider to request randomness (Placeholder)
            // This is where you would call Switchboard's request_randomness instruction
            // Example (conceptual, actual CPI is more complex):
            // switchboard_program.request_randomness(
            //     CpiContext::new_with_signer(
            //         ctx.accounts.switchboard_program.to_account_info(),
            //         RequestRandomnessAccounts { ... }, // Populate with actual accounts
            //         signer_seeds // If PDA is a signer for VRF client
            //     ),
            //     params // VRF parameters
            // )?;
            msg!("Placeholder: CPI call to VRF provider to request randomness");

            // Assuming vrf_client key is what we need to store, or a new key generated by request.
            // For Switchboard, the VRF account (client) itself is the key.
            // If lottery_account.vrf_client is already set (e.g. during lottery creation), use that.
            // Otherwise, it might be one of the accounts passed into this instruction.
            // For now, let's assume it's available or passed in, and we're storing it.
            // lottery_account.vrf_request_key = Some(ctx.accounts.vrf.key()); // Example
            
            // For this example, let's assume the vrf_client field on lottery_account
            // was set when the lottery was created, and that's our request key.
            // Ensure draw time has passed for Open -> AwaitingRandomness (via Drawing)
            if clock.unix_timestamp < lottery_account.draw_time {
                return Err(LotteryError::DrawTimeNotReached.into());
            }
            if lottery_account.vrf_client.is_none() {
                return Err(LotteryError::VrfClientNotSet.into());
            }
            lottery_account.vrf_request_key = lottery_account.vrf_client;
            lottery_account.randomness_fulfilled = false;
            actual_next_state = LotteryState::AwaitingRandomness; // Override target state
        },
        (LotteryState::Drawing, LotteryState::AwaitingRandomness) => {
            // This transition is primarily handled by the Open -> Drawing case above,
            // which directly sets to AwaitingRandomness.
            // This arm could be for an explicit admin call if Drawing was an intermediate manual step.
            // For now, we assume Open -> Drawing automatically implies VRF request and AwaitingRandomness state.
             msg!("Transitioning from Drawing to AwaitingRandomness");
            // Ensure VRF request fields are set if not already
            if lottery_account.vrf_client.is_none() {
                return Err(LotteryError::VrfClientNotSet.into());
            }
            lottery_account.vrf_request_key = lottery_account.vrf_client;
            lottery_account.randomness_fulfilled = false;
            // Placeholder for CPI to VRF provider if not done in a previous step that led to "Drawing"
            msg!("Placeholder: CPI call to VRF provider to request randomness (if admin re-triggers from Drawing)");
        },
        (LotteryState::AwaitingRandomness, LotteryState::Completed) => {
            // This transition should ideally happen in settle_randomness instruction
            // after VRF callback is processed by the settle_randomness instruction.
            // Admin manual transition here implies randomness was fulfilled externally or is being forced.
            if !lottery_account.randomness_fulfilled {
                return Err(LotteryError::RandomnessNotFulfilled.into());
            }
            if lottery_account.winning_ticket.is_none() {
                 // It's possible winner is not yet selected if admin forces Completed state.
                 // select_winner instruction handles winner selection based on randomness.
                 // So, this constraint might be too strict for a manual override to Completed.
                 // However, if Completed implies winner is known, then it's okay.
                 // For now, assuming Completed means randomness is there, winner selection is next.
                 // The select_winner instruction has a constraint `lottery_account.winning_ticket.is_none()`
                 // msg!("Warning: Transitioning to Completed but winning ticket not yet selected via select_winner instruction.");
            }
            lottery_account.completed_at = Some(clock.unix_timestamp);
        },
        (LotteryState::AwaitingRandomness, LotteryState::Expired) => {
            // Admin manually expiring a lottery if VRF callback timed out.
            // Define a reasonable timeout, e.g., 1 hour (3600 seconds) after draw_time.
            const VRF_CALLBACK_TIMEOUT_SECONDS: i64 = 3600;
            if clock.unix_timestamp < lottery_account.draw_time.saturating_add(VRF_CALLBACK_TIMEOUT_SECONDS) {
                return Err(LotteryError::VrfCallbackNotTimedOut.into());
            }
            if lottery_account.randomness_fulfilled {
                // If randomness was somehow fulfilled but admin is expiring, this is unusual.
                // Could log a warning or prevent, but admin override might be needed.
                msg!("Warning: Expiring lottery even though randomness was fulfilled.");
            }
            lottery_account.completed_at = Some(clock.unix_timestamp);
            msg!("Lottery expired due to VRF callback timeout.");
        },
        (_, LotteryState::Cancelled) => {
            if !current_state.can_cancel() {
                return Err(LotteryError::InvalidCancellation.into());
            }
            lottery_account.completed_at = Some(clock.unix_timestamp);
        }
        _ => {
            // Disallow other transitions not explicitly handled or covered by can_transition_to
            if actual_next_state == next_state_param { // if not overridden by specific logic
                 return Err(LotteryError::InvalidStateTransition.into());
            }
        }
    }

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
