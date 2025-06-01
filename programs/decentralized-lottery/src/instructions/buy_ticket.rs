use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer, TokenAccount};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::state::treasury::GlobalConfig;
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
        constraint = !lottery_account.is_claimed @ LotteryError::LotteryAlreadyClaimed,
        has_one = global_config
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        init,
        payer = user,
        space = TicketAccount::ACCOUNT_SIZE,
        seeds = [
            b"ticket", 
            lottery_account.key().as_ref(), 
            &(lottery_account.last_ticket_id + 1).to_le_bytes()
        ],
        bump
    )]
    /// The ticket account representing a single ticket purchase in the lottery.
    /// PDA Derivation Explanation:
    /// - Seed prefix: b"ticket" - A static identifier for ticket accounts.
    /// - Seed 1: lottery_account.key().as_ref() - The public key of the associated lottery account, linking this ticket to a specific lottery.
    /// - Seed 2: lottery_account.last_ticket_id.to_le_bytes() - The ticket ID as little-endian bytes, ensuring each ticket for a lottery has a unique address.
    /// - Bump: Automatically determined by Anchor to find a valid Program Derived Address (PDA).
    /// Rationale: This derivation ensures that each ticket is uniquely tied to a specific lottery and ticket ID, preventing collisions and allowing for efficient lookup and validation during prize claiming or refund processes.
    pub ticket_account: Account<'info, TicketAccount>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// User's USDC token account
    #[account(
        mut,
        constraint = user_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount,
        constraint = user_token_account.owner == user.key() @ LotteryError::InvalidTokenAccount
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    /// Lottery's USDC token account
    #[account(
        mut,
        constraint = lottery_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<BuyTicket>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let ticket_account = &mut ctx.accounts.ticket_account;
    let user = &ctx.accounts.user;
    
    // Additional validation: Check if draw time has passed
    let current_time = Clock::get()?.unix_timestamp;
    if current_time >= lottery_account.draw_time {
        return Err(LotteryError::LotteryExpired.into());
    }

    // Validate ticket price against buyer's token balance
    let ticket_cost = lottery_account.ticket_price;
    if ctx.accounts.user_token_account.amount < ticket_cost {
        return Err(LotteryError::InsufficientFunds.into());
    }

    let ticket_id = lottery_account.last_ticket_id;

    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.lottery_token_account.to_account_info(),
            authority: user.to_account_info(),
        },
    );
    token::transfer(transfer_ctx, ticket_cost)?;

    ticket_account.lottery = lottery_account.key();
    ticket_account.id = ticket_id;
    ticket_account.buyer = user.key();
    ticket_account.is_claimed = false;
    let bump = ctx.bumps.ticket_account;
    ticket_account.bump = bump;

    // Use safe_add to prevent overflow, though unlikely in realistic scenarios
    lottery_account.total_tickets = safe_add(
        lottery_account.total_tickets,
        1,
    ).map_err(|_| LotteryError::ArithmeticOverflow)?;
    
    lottery_account.prize_pool = safe_add(
        lottery_account.prize_pool,
        ticket_cost,
    ).map_err(|_| LotteryError::ArithmeticOverflow)?;

    lottery_account.last_ticket_id = safe_add(
        lottery_account.last_ticket_id,
        1,
    ).map_err(|_| LotteryError::ArithmeticOverflow)?;

    emit!(TicketPurchased {
        lottery_id: lottery_account.key(),
        ticket_id: ticket_id,
        buyer: user.key(),
        number_of_tickets: 1,
        total_cost: ticket_cost,
        timestamp: current_time,
    });

    Ok(())
} 