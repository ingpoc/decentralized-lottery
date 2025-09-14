use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::errors::LotteryError;
use crate::events::{LotteryWinnerDetermined, LotteryStateChanged};
use crate::utils;

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

    /// CHECK: Winning ticket account is validated in instruction logic
    #[account(mut)]
    pub winning_ticket_account: AccountInfo<'info>,

    pub system_program: Program<'info, System>,
}

pub fn select_winner_handler(ctx: Context<SelectWinner>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let clock = Clock::get()?;

    // Ensure randomness is available (double check constraint, though belt-and-suspenders)
    let vrf_randomness_bytes = lottery_account.vrf_randomness.ok_or(LotteryError::RandomnessNotAvailable)?;

    if lottery_account.total_tickets == 0 {
        return Err(LotteryError::NoTicketsSold.into());
    }

    // Convert randomness bytes to a u64.
    let mut randomness_u64_bytes = [0u8; 8];
    randomness_u64_bytes.copy_from_slice(&vrf_randomness_bytes[0..8]);
    let random_value = u64::from_le_bytes(randomness_u64_bytes);

    // Select winning ticket ID (1-based)
    let winning_ticket_id = (random_value % lottery_account.total_tickets) + 1;

    // Derive the winning ticket PDA
    let winning_ticket_pda = utils::get_ticket_pda_pubkey(
        &lottery_account.key(),
        winning_ticket_id
    )?;

    // Validate the provided ticket account matches the winning ticket
    require!(winning_ticket_pda == ctx.accounts.winning_ticket_account.key(), LotteryError::InvalidWinningTicket);

    // Load and deserialize the winning ticket account
    let ticket_data = ctx.accounts.winning_ticket_account.try_borrow_data()?;
    let winning_ticket: TicketAccount = TicketAccount::try_deserialize(&mut ticket_data.as_ref())?;
    drop(ticket_data);

    // Validate ticket belongs to this lottery
    require!(winning_ticket.lottery == lottery_account.key(), LotteryError::TicketNotForThisLottery);

    // Get the actual winner's public key
    let winner_pubkey = winning_ticket.buyer;

    lottery_account.winning_ticket = Some(winning_ticket_pda);
    lottery_account.set_is_prize_pool_locked(true); // Lock prize pool now that winner is selected
    
    // Mark lottery as completed
    lottery_account.mark_completed(clock.unix_timestamp);

    // Emit winner determination event with actual winner address
    emit!(LotteryWinnerDetermined {
        lottery_id: lottery_account.key(),
        previous_state: LotteryState::Completed,
        new_state: LotteryState::Completed,
        winner: winner_pubkey, // Actual winner's wallet address
        randomness: random_value,
        timestamp: clock.unix_timestamp,
    });

    // Emit state change event
    emit!(LotteryStateChanged {
        lottery_id: lottery_account.key(),
        previous_state: LotteryState::Completed,
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
