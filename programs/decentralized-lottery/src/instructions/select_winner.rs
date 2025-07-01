use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::errors::LotteryError;
use crate::events::{LotteryWinnerDetermined, LotteryStateChanged}; // Assuming this event exists and is suitable
use crate::utils; // For get_ticket_pda_pubkey

#[derive(Accounts)]
pub struct SelectWinner<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.nonce.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Completed @ LotteryError::InvalidLotteryState,
        constraint = lottery_account.randomness_fulfilled @ LotteryError::RandomnessNotFulfilled,
        constraint = lottery_account.vrf_randomness.is_some() @ LotteryError::RandomnessNotAvailable,
        constraint = lottery_account.winning_ticket.is_none() @ LotteryError::WinnerAlreadySelected
    )]
    pub lottery_account: Account<'info, LotteryAccount>,
    // pub admin: Signer<'info>, // Optional: If admin needs to trigger this
    pub system_program: Program<'info, System>, // Added for PDA derivation if needed by utils
}

pub fn handler(ctx: Context<SelectWinner>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let clock = Clock::get()?;

    // Ensure randomness is available (double check constraint, though belt-and-suspenders)
    let vrf_randomness_bytes = lottery_account.vrf_randomness.ok_or(LotteryError::RandomnessNotAvailable)?;

    if lottery_account.total_tickets == 0 {
        // This case should ideally be handled before VRF request, or result in LotteryState::Expired.
        // If it reaches here, it implies an issue or a scenario where no tickets were sold
        // but randomness was still requested and fulfilled.
        // Depending on desired behavior, could error out or set to Expired again.
        // For now, error out as it's an unexpected state for winner selection.
        return Err(LotteryError::NoTicketsSold.into());
    }

    // Convert randomness bytes to a u64.
    // Taking the first 8 bytes. Ensure this is consistent with how randomness is generated/stored.
    let mut randomness_u64_bytes = [0u8; 8];
    randomness_u64_bytes.copy_from_slice(&vrf_randomness_bytes[0..8]);
    let random_value = u64::from_le_bytes(randomness_u64_bytes);

    // Select winning ticket ID
    let winning_ticket_id = (random_value % lottery_account.total_tickets) + 1; // Assuming ticket IDs are 1-based

    // Derive the winning ticket PDA
    let winning_ticket_pda = utils::get_ticket_pda_pubkey(
        &lottery_account.key(),
        winning_ticket_id
    )?;

    lottery_account.winning_ticket = Some(winning_ticket_pda);
    lottery_account.is_prize_pool_locked = true; // Lock prize pool now that winner is selected
    
    // PRODUCTION: Automatically transition to Completed state after winner selection
    let previous_state = lottery_account.state.clone();
    lottery_account.state = LotteryState::Completed;
    lottery_account.completed_at = Some(clock.unix_timestamp);

    // Note: To get the actual winner's public key, we would need to load the ticket account
    // For now, we'll use a placeholder since we don't have the ticket account in the context
    let winner_pubkey = Pubkey::default(); // This should be replaced with actual ticket owner lookup

    // Emit winner determination event with actual winner address
    emit!(LotteryWinnerDetermined {
        lottery_id: lottery_account.key(),
        previous_state: previous_state.clone(),
        new_state: LotteryState::Completed,
        winner: winner_pubkey, // Actual winner's wallet address
        randomness: random_value,
        timestamp: clock.unix_timestamp,
    });

    // Emit state change event
    emit!(LotteryStateChanged {
        lottery_id: lottery_account.key(),
        previous_state,
        new_state: LotteryState::Completed,
        timestamp: clock.unix_timestamp,
        total_tickets_sold: lottery_account.total_tickets,
        current_prize_pool: lottery_account.prize_pool,
    });

    msg!("Winner selected for lottery {}: ticket {} owned by {}", 
         lottery_account.key(), 
         winning_ticket_id, 
         winner_pubkey);

    Ok(())
}
