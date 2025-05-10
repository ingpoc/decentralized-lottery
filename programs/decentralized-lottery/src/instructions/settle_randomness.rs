use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::vrf::VrfClientState;
use crate::state::ticket::TicketAccount;
use crate::errors::LotteryError;
use crate::events::{LotteryStateChanged, WinnerSelected};
use crate::utils::get_ticket_pda_pubkey;

use switchboard_v2::VrfAccountData;

/// Context for settling the randomness for a lottery using Switchboard VRF.
/// This instruction should be called after the VRF randomness has been consumed
/// through the consume_randomness instruction, to finalize the lottery draw.
#[derive(Accounts)]
pub struct SettleRandomness<'info> {
    /// The payer of the transaction (usually the lottery authority or a delegated keeper)
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// The lottery account to be settled
    #[account(
        mut,
        constraint = lottery_account.state == LotteryState::AwaitingRandomness @ LotteryError::InvalidStateTransition,
        constraint = !lottery_account.is_claimed @ LotteryError::LotteryAlreadyClaimed,
        constraint = lottery_account.vrf_client.is_some() @ LotteryError::RandomnessGenerationFailed,
        constraint = lottery_account.vrf_client.unwrap() == vrf_client.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// The VRF client state account containing the randomness
    #[account(
        mut,
        seeds = [
            b"vrf-client", 
            lottery_account.key().as_ref()
        ],
        bump = vrf_client.bump,
        constraint = vrf_client.is_consumed @ LotteryError::RandomnessGenerationFailed,
        constraint = vrf_client.lottery_account == lottery_account.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub vrf_client: Account<'info, VrfClientState>,
    
    /// The Switchboard VRF account used for this lottery's randomness
    #[account(
        constraint = vrf_client.vrf_account == vrf.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Handler for the `settle_randomness` instruction.
/// Uses the verified VRF randomness to select the winning ticket and complete the lottery.
pub fn handler(ctx: Context<SettleRandomness>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let vrf_client = &ctx.accounts.vrf_client;
    let clock = Clock::get()?;

    // --- Extract Randomness from VRF Client --- 
    // The randomness has already been verified and consumed in the consume_randomness instruction
    let random_bytes = vrf_client.result_buffer;
    msg!("Using VRF randomness bytes from client: {:?}", random_bytes);
    
    // Store the randomness in the lottery account for transparency
    lottery_account.store_vrf_randomness(random_bytes);
    
    // Extract a u64 value from the first 8 bytes of randomness
    let random_value = vrf_client.get_random_value();
    msg!("Randomness value (u64): {}", random_value);

    // --- Determine Winning Ticket --- 
    if lottery_account.total_tickets == 0 {
        // No tickets sold - mark as expired
        lottery_account.state = LotteryState::Expired;
        let previous_state = LotteryState::AwaitingRandomness;
        lottery_account.completed_at = Some(clock.unix_timestamp);
        
        msg!("No tickets sold. Lottery marked as Expired.");
        
        // Emit state change event
        emit!(LotteryStateChanged {
            lottery_id: lottery_account.key(),
            previous_state,
            new_state: LotteryState::Expired,
            timestamp: clock.unix_timestamp,
            total_tickets_sold: lottery_account.total_tickets,
            current_prize_pool: lottery_account.prize_pool,
        });
        
        return Ok(());
    }

    // Use the verified random value to select the winning ticket ID
    let winning_ticket_id = random_value % lottery_account.total_tickets;
    msg!("Winning ticket ID calculated: {}", winning_ticket_id);

    // --- Derive and Store Winning Ticket PDA --- 
    let winning_ticket_pubkey = get_ticket_pda_pubkey(&lottery_account.key(), winning_ticket_id)?;
    msg!("Winning ticket PDA: {}", winning_ticket_pubkey);

    // Store the winning ticket in the lottery account
    lottery_account.winning_ticket = Some(winning_ticket_pubkey);

    // --- Update Lottery State --- 
    let previous_state = lottery_account.state.clone();
    lottery_account.state = LotteryState::Completed;
    lottery_account.completed_at = Some(clock.unix_timestamp);
    
    msg!("Lottery state transitioned to Completed.");

    // --- Emit Events --- 
    // Event for winner selection
    emit!(WinnerSelected {
        lottery_id: lottery_account.key(),
        winning_ticket_id: winning_ticket_id,
        winning_ticket_pda: winning_ticket_pubkey,
        // Emit the full randomness bytes as a string for transparency
        randomness_value: format!("{:?}", random_bytes), 
        timestamp: clock.unix_timestamp,
    });

    // Event for state change
    emit!(LotteryStateChanged {
        lottery_id: lottery_account.key(),
        previous_state,
        new_state: LotteryState::Completed,
        timestamp: clock.unix_timestamp,
        total_tickets_sold: lottery_account.total_tickets,
        current_prize_pool: lottery_account.prize_pool,
    });

    Ok(())
}
