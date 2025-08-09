use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer, Token};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::errors::LotteryError;
use crate::events::TicketPurchased;

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(mut)]
    pub lottery_account: Box<Account<'info, LotteryAccount>>,

    #[account(
        init,
        payer = user,
        space = TicketAccount::ACCOUNT_SIZE,
        seeds = [b"ticket", lottery_account.key().as_ref(), &(lottery_account.last_ticket_id + 1).to_le_bytes()],
        bump
    )]
    pub ticket_account: Box<Account<'info, TicketAccount>>,

    #[account()]
    pub global_config: AccountInfo<'info>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub user_token_account: AccountInfo<'info>,

    #[account(mut)]
    pub lottery_token_account: AccountInfo<'info>,

    pub usdc_mint: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn buy_ticket_handler(ctx: Context<BuyTicket>) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // Check timing and state
    require!(ctx.accounts.lottery_account.state == LotteryState::Open, LotteryError::LotteryNotOpen);
    require!(current_time < ctx.accounts.lottery_account.draw_time, LotteryError::LotteryNotOpen);

    let ticket_id = ctx.accounts.lottery_account.last_ticket_id.checked_add(1)
        .ok_or(LotteryError::ArithmeticOverflow)?;
    let ticket_price = ctx.accounts.lottery_account.ticket_price;
    
    // Update lottery state
    ctx.accounts.lottery_account.last_ticket_id = ticket_id;
    ctx.accounts.lottery_account.total_tickets = ctx.accounts.lottery_account.total_tickets
        .checked_add(1)
        .ok_or(LotteryError::ArithmeticOverflow)?;
    ctx.accounts.lottery_account.prize_pool = ctx.accounts.lottery_account.prize_pool
        .checked_add(ticket_price)
        .ok_or(LotteryError::ArithmeticOverflow)?;

    // Initialize ticket
    ctx.accounts.ticket_account.lottery = ctx.accounts.lottery_account.key();
    ctx.accounts.ticket_account.buyer = ctx.accounts.user.key();
    ctx.accounts.ticket_account.id = ticket_id;
    ctx.accounts.ticket_account.is_claimed = false;
    ctx.accounts.ticket_account.bump = ctx.bumps.ticket_account;
    
    // Transfer tokens
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.lottery_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        }
    );
    token::transfer(transfer_ctx, ticket_price)?;

    emit!(TicketPurchased {
        lottery_id: ctx.accounts.lottery_account.key(),
        ticket_id,
        buyer: ctx.accounts.user.key(),
        number_of_tickets: 1,
        total_cost: ticket_price,
        timestamp: current_time,
    });

    Ok(())
}
