use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::treasury::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::LotteryStateChanged;

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

    /// The lottery's token account for prize pool
    #[account(
        mut,
        constraint = lottery_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,

    /// Placeholder: Depending on the VRF provider, specific accounts (VRF account, Oracle queue, etc.)
    /// would be needed here when transitioning to Drawing/AwaitingRandomness.
    /// pub vrf: AccountInfo<'info>,
    /// pub oracle_queue: AccountInfo<'info>,
    /// pub vrf_request_account: AccountInfo<'info>, // The account to store request state

    #[account(mut)]
    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<TransitionState>, next_state: LotteryState) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let current_state = lottery_account.state.clone();
    let clock = Clock::get()?;

    // Validate state transition
    if !current_state.can_transition_to(&next_state) {
        return Err(LotteryError::InvalidStateTransition.into());
    }

    // Additional validations based on specific transitions
    match (&current_state, &next_state) {
        (LotteryState::Created, LotteryState::Open) => {
            // Validate draw time hasn't passed
            if lottery_account.draw_time <= clock.unix_timestamp {
                return Err(LotteryError::InvalidDrawTime.into());
            }

            // No need to validate prize pool is funded - it will build from ticket sales
            
            // Reset prize pool lock
            lottery_account.is_prize_pool_locked = false;
        },
        (LotteryState::Open, LotteryState::Drawing) => {
            // Validate draw time has been reached
            if lottery_account.draw_time > clock.unix_timestamp {
                return Err(LotteryError::InvalidDrawTime.into());
            }

            // Validate there are tickets sold
            if lottery_account.total_tickets == 0 {
                // If no tickets sold, we should expire the lottery instead
                lottery_account.state = LotteryState::Expired;
                
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

            // Lock prize pool
            lottery_account.is_prize_pool_locked = true;
            
            // Placeholder: Initiate VRF request CPI call here using oracle accounts
            // This would typically involve calling the VRF provider's program
            // E.g., switchboard_program.request_randomness(...);
            // Need to pass lottery account key, payer (admin), VRF accounts from context
            msg!("Placeholder: Initiating VRF request...");
            
            // Store the VRF request account pubkey (passed in context) if required by oracle
            // lottery_account.vrf_request_account = Some(ctx.accounts.vrf_request_account.key());
            
            // Set state to AwaitingRandomness
            lottery_account.state = LotteryState::AwaitingRandomness;
            msg!("Lottery state transitioned to AwaitingRandomness");

            // Emit state change event for AwaitingRandomness
            emit!(LotteryStateChanged {
                lottery_id: lottery_account.key(),
                previous_state: current_state,
                new_state: LotteryState::AwaitingRandomness,
                timestamp: clock.unix_timestamp,
                total_tickets_sold: lottery_account.total_tickets,
                current_prize_pool: lottery_account.prize_pool,
            });
            
            // Return early as state transition is handled within this block
            return Ok(());
        },
        _ => {}
    }

    // Update state
    lottery_account.state = next_state.clone();

    // Emit state change event
    emit!(LotteryStateChanged {
        lottery_id: lottery_account.key(),
        previous_state: current_state,
        new_state: next_state,
        timestamp: clock.unix_timestamp,
        total_tickets_sold: lottery_account.total_tickets,
        current_prize_pool: lottery_account.prize_pool,
    });

    Ok(())
} 