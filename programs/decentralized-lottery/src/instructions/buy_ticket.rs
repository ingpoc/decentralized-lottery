use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};
use anchor_spl::associated_token::AssociatedToken;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::state::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::{TicketPurchased, LotteryStateChanged};

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.created_at.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Open @ LotteryError::LotteryNotOpen
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
    pub ticket_account: Account<'info, TicketAccount>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ LotteryError::InvalidTokenAccount,
        constraint = user_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount
    )]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = usdc_mint,
        associated_token::authority = lottery_account,
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,

    #[account(constraint = usdc_mint.key() == global_config.usdc_mint)]
    pub usdc_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<BuyTicket>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let ticket_account = &mut ctx.accounts.ticket_account;
    let user = &ctx.accounts.user;
    let clock = Clock::get()?;
    
    // Additional validation: Check if draw time has passed
    if clock.unix_timestamp >= lottery_account.draw_time && lottery_account.state == LotteryState::Open {
        // Don't change state here. Emit an event to suggest admin action.
        // Prevent further ticket sales.
        emit!(LotteryStateChanged { // Using LotteryStateChanged event for consistency, though state doesn't change here.
                                    // Alternatively, a new specific event like DrawTimePassed could be created.
            lottery_id: lottery_account.key(),
            previous_state: lottery_account.state.clone(), // Still Open
            new_state: lottery_account.state.clone(), // Still Open, but should be transitioned by admin
            timestamp: clock.unix_timestamp,
            total_tickets_sold: lottery_account.total_tickets,
            current_prize_pool: lottery_account.prize_pool,
        });
        msg!("Draw time has passed for lottery {}. Ticket sales are now closed. Admin should transition state.", lottery_account.key());
        return Err(LotteryError::LotteryNotOpen.into()); // Or a more specific error like LotteryDrawTimePassed
    }

    if lottery_account.state != LotteryState::Open {
        return Err(LotteryError::LotteryNotOpen.into());
    }

    let ticket_id = lottery_account.last_ticket_id.checked_add(1).ok_or(LotteryError::ArithmeticOverflow)?;
    lottery_account.last_ticket_id = ticket_id;
    lottery_account.total_tickets = lottery_account.total_tickets.checked_add(1).ok_or(LotteryError::ArithmeticOverflow)?;
    lottery_account.prize_pool = lottery_account.prize_pool.checked_add(lottery_account.ticket_price).ok_or(LotteryError::ArithmeticOverflow)?;

    // Initialize ticket account
    ticket_account.lottery = lottery_account.key();
    ticket_account.buyer = user.key();
    ticket_account.id = ticket_id;
    ticket_account.is_claimed = false;
    
    // Transfer USDC from user to lottery
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.lottery_token_account.to_account_info(),
        authority: user.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, lottery_account.ticket_price)?;

    emit!(TicketPurchased {
        lottery_id: lottery_account.key(),
        ticket_id,
        buyer: user.key(),
        number_of_tickets: 1,
        total_cost: lottery_account.ticket_price,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
