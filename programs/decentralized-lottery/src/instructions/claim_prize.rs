use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::state::treasury::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::PrizeClaimed; // Assuming this event will be created
use crate::utils::safe_mul_div; // Ensure this is imported

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    #[account(
        mut, // Lottery account needs mutation to mark as claimed
        constraint = lottery_account.state == LotteryState::Completed @ LotteryError::InvalidStateTransition,
        constraint = !lottery_account.is_claimed @ LotteryError::LotteryAlreadyClaimed,
        has_one = global_config, // Ensure lottery links to the provided global config
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        mut, // Ticket account needs mutation to mark as claimed
        // Verify this ticket PDA matches the one stored in the lottery account
        constraint = ticket_account.key() == lottery_account.winning_ticket.unwrap_or_default() @ LotteryError::InvalidWinningTicket,
        // Verify the ticket hasn't already been claimed (double check)
        constraint = !ticket_account.is_claimed @ LotteryError::TicketAlreadyClaimed,
        // Verify the ticket belongs to this lottery
        constraint = ticket_account.lottery == lottery_account.key() @ LotteryError::TicketNotForThisLottery,
        // Verify the signer is the buyer stored on the ticket
        constraint = ticket_account.buyer == winner.key() @ LotteryError::UnauthorizedAccess, 
        // Define seeds for verification (even though not initializing)
        seeds = [
            b"ticket", 
            lottery_account.key().as_ref(), 
            &ticket_account.id.to_le_bytes()
        ],
        bump = ticket_account.bump,
    )]
    /// The ticket account of the winning ticket.
    /// PDA Derivation Validation:
    /// - Ensures the provided ticket account matches the derived PDA based on lottery key and ticket ID.
    /// - Validates that only the legitimate winning ticket, as recorded in the lottery account, can claim the prize.
    pub ticket_account: Account<'info, TicketAccount>,

    #[account(
        // No constraints needed here as admin/authority is checked via global_config
        seeds = [b"global_config"], 
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(
        mut,
        // Ensure the treasury token account matches the one in global config
        address = global_config.treasury_token_account @ LotteryError::InvalidTokenAccount 
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        // Ensure the lottery token account holds the prize pool
        // constraint = lottery_token_account.owner == lottery_account.key() @ LotteryError::InvalidAccountOwner, // Authority is lottery_account PDA
        constraint = lottery_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        // Ensure winner's token account can receive the prize
        constraint = winner_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount,
        constraint = winner_token_account.owner == winner.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub winner_token_account: Account<'info, TokenAccount>,

    #[account(mut)] // Winner must sign to prove ownership and receive funds
    pub winner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClaimPrize>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let ticket_account = &mut ctx.accounts.ticket_account;
    let global_config = &ctx.accounts.global_config;
    let clock = Clock::get()?;

    // Additional validation: Ensure winning_ticket is set in lottery_account
    if lottery_account.winning_ticket.is_none() {
        return Err(LotteryError::NoWinnerSelected.into());
    }

    // Additional validation: Ensure prize pool is greater than zero
    if lottery_account.prize_pool == 0 {
        return Err(LotteryError::EmptyPrizePool.into());
    }

    // Additional validation: Ensure lottery_token_account has sufficient balance
    if ctx.accounts.lottery_token_account.amount < lottery_account.prize_pool {
        return Err(LotteryError::InsufficientPrizeFunds.into());
    }

    let total_prize = lottery_account.prize_pool;
    let fee_basis_points = global_config.treasury_fee_percentage as u64;
    
    let treasury_fee = safe_mul_div(total_prize, fee_basis_points, 10000).map_err(|_| LotteryError::ArithmeticOverflow)?;
    msg!("Calculated treasury fee: {}", treasury_fee);
    
    let winner_payout = total_prize.checked_sub(treasury_fee).ok_or(LotteryError::ArithmeticOverflow)?;
    msg!("Calculated winner payout: {}", winner_payout);

    // Derive the lottery PDA bump within the handler
    let lottery_type_string = lottery_account.lottery_type.to_string();
    let draw_time_bytes = lottery_account.draw_time.to_le_bytes();
    let (_lottery_pda, lottery_bump) = Pubkey::find_program_address(
        &[
            b"lottery".as_ref(),
            lottery_type_string.as_bytes(),
            draw_time_bytes.as_ref()
        ],
        ctx.program_id
    );

    let lottery_seeds = &[ 
        b"lottery".as_ref(),
        lottery_type_string.as_bytes(),
        draw_time_bytes.as_ref(),
        &[lottery_bump] // Use the derived bump
    ];
    let signer_seeds = &[&lottery_seeds[..]];

    // Transfer treasury fee
    if treasury_fee > 0 {
        let fee_transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.lottery_token_account.to_account_info(),
                to: ctx.accounts.treasury_token_account.to_account_info(),
                authority: lottery_account.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(fee_transfer_ctx, treasury_fee).map_err(|_| LotteryError::TokenTransferFailed)?;
        msg!("Transferred fee to treasury");
    }

    // Transfer winner payout
    if winner_payout > 0 {
        let payout_transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.lottery_token_account.to_account_info(),
                to: ctx.accounts.winner_token_account.to_account_info(),
                authority: lottery_account.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(payout_transfer_ctx, winner_payout).map_err(|_| LotteryError::TokenTransferFailed)?;
        msg!("Transferred payout to winner");
    }

    lottery_account.is_claimed = true;
    ticket_account.is_claimed = true;
    msg!("Marked lottery and ticket as claimed");

    emit!(PrizeClaimed {
        lottery_id: lottery_account.key(),
        ticket_id: ticket_account.id,
        winner: ctx.accounts.winner.key(),
        prize_pool: total_prize,
        treasury_fee: treasury_fee,
        winner_payout: winner_payout,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
} 