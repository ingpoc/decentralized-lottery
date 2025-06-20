use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer, Token};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::state::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::PrizeClaimed;

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.created_at.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Completed @ LotteryError::InvalidLotteryState,
        constraint = lottery_account.winning_ticket.is_some() @ LotteryError::NoWinnerSelected,
        constraint = lottery_account.winning_ticket.unwrap() == ticket_account.key() @ LotteryError::InvalidWinningTicket,
        constraint = !lottery_account.is_claimed @ LotteryError::LotteryAlreadyClaimed
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        mut,
        constraint = !ticket_account.is_claimed @ LotteryError::TicketAlreadyClaimed,
        constraint = ticket_account.lottery == lottery_account.key() @ LotteryError::TicketNotForThisLottery,
        constraint = ticket_account.buyer == winner.key() @ LotteryError::UnauthorizedAccess // Winner is the signer
    )]
    pub ticket_account: Account<'info, TicketAccount>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub winner: Signer<'info>, // This is the buyer of the winning ticket

    /// CHECK: Lottery's token account that holds the prize pool
    #[account(mut)]
    pub lottery_token_account: AccountInfo<'info>,

    /// CHECK: Winner's USDC token account 
    #[account(mut)]
    pub winner_token_account: AccountInfo<'info>,

    /// CHECK: Treasury USDC token account
    #[account(
        mut,
        address = global_config.treasury_token_account @ LotteryError::InvalidTokenAccount
    )]
    pub treasury_token_account: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimPrize>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let ticket_account = &mut ctx.accounts.ticket_account;
    let global_config = &ctx.accounts.global_config;
    let clock = Clock::get()?;

    // Calculate treasury fee and winner payout
    let treasury_fee = lottery_account.prize_pool
        .checked_mul(global_config.treasury_fee_percentage as u64)
        .ok_or(LotteryError::ArithmeticOverflow)?
        .checked_div(10000) // basis points (e.g., 250 for 2.5%)
        .ok_or(LotteryError::ArithmeticOverflow)?;

    let winner_payout = lottery_account.prize_pool
        .checked_sub(treasury_fee)
        .ok_or(LotteryError::ArithmeticOverflow)?;

    // Signer seeds for PDA to authorize token transfers
    let authority_seeds: &[&[&[u8]]] = &[&[
        b"lottery",
        lottery_account.authority.as_ref(),
        &lottery_account.created_at.to_le_bytes(),
        &[ctx.bumps.lottery_account], // Use the bump from the lottery_account
    ]];

    // Transfer treasury fee
    if treasury_fee > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.lottery_token_account.to_account_info(),
            to: ctx.accounts.treasury_token_account.to_account_info(),
            authority: lottery_account.to_account_info(), // PDA is the authority
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, authority_seeds);
        token::transfer(cpi_ctx, treasury_fee)?;
    }

    // Transfer winner payout
    if winner_payout > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.lottery_token_account.to_account_info(),
            to: ctx.accounts.winner_token_account.to_account_info(),
            authority: lottery_account.to_account_info(), // PDA is the authority
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, authority_seeds);
        token::transfer(cpi_ctx, winner_payout)?;
    }

    lottery_account.is_claimed = true;
    ticket_account.is_claimed = true;

    msg!("Prize claimed for lottery {} by ticket {}", lottery_account.key(), ticket_account.key());

    emit!(PrizeClaimed {
        lottery_id: lottery_account.key(),
        ticket_id: ticket_account.id, // Assuming TicketAccount has an 'id' field for the numerical ID
        winner: ticket_account.buyer,
        prize_pool: lottery_account.prize_pool,
        treasury_fee,
        winner_payout,
        timestamp: clock.unix_timestamp,
    });
    Ok(())
}
