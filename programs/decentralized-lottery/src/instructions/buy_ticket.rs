use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::errors::LotteryError;
use crate::events::TicketPurchased;
use crate::utils::safe_add;

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(
        mut,
        seeds = [
            b"lottery",
            lottery_account.lottery_type.to_string().as_bytes(),
            &lottery_account.draw_time.to_le_bytes()
        ],
        bump,
        constraint = lottery_account.state == LotteryState::Open @ LotteryError::LotteryNotOpen,
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        mut,
        constraint = user_token_account.mint == lottery_token_account.mint @ LotteryError::InvalidTokenAccount,
        constraint = user_token_account.owner == buyer.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = lottery_token_account.owner == lottery_account.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<BuyTicket>, number_of_tickets: u64) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    
    // Validate lottery state and ticket purchase
    if lottery_account.total_tickets >= 10_000 {
        return Err(LotteryError::TicketPurchaseLimitReached.into());
    }

    // Calculate total cost
    let total_cost = lottery_account
        .ticket_price
        .checked_mul(number_of_tickets)
        .ok_or(LotteryError::SafeMathError)?;

    // Transfer tokens from user to lottery account
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.lottery_token_account.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, total_cost)?;

    // Update lottery state
    lottery_account.total_tickets = safe_add(
        lottery_account.total_tickets,
        number_of_tickets,
    )?;
    
    // Update prize pool - all ticket purchases contribute to the prize pool
    lottery_account.prize_pool = safe_add(
        lottery_account.prize_pool,
        total_cost,
    )?;

    // Emit ticket purchase event
    emit!(TicketPurchased {
        lottery_id: lottery_account.key(),
        buyer: ctx.accounts.buyer.key(),
        number_of_tickets,
        total_cost,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
} 