use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, Transfer, TokenAccount};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::state::treasury::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::TicketRefunded; // Assuming this event will be created

#[derive(Accounts)]
pub struct ClaimRefund<'info> {
    #[account(
        // Need access to lottery state and ticket price
        // No mut needed if not decrementing counters
        constraint = lottery_account.state == LotteryState::Cancelled || lottery_account.state == LotteryState::Expired 
            @ LotteryError::InvalidStateForRefund,
        has_one = global_config
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        mut, // Ticket needs mutation to mark refunded (is_claimed = true)
        constraint = ticket_account.lottery == lottery_account.key() @ LotteryError::TicketNotForThisLottery,
        constraint = ticket_account.buyer == buyer.key() @ LotteryError::UnauthorizedAccess,
        constraint = !ticket_account.is_claimed @ LotteryError::TicketAlreadyClaimed,
        seeds = [
            b"ticket",
            lottery_account.key().as_ref(),
            &ticket_account.id.to_le_bytes()
        ],
        bump = ticket_account.bump
    )]
    pub ticket_account: Account<'info, TicketAccount>,

    #[account(
        seeds = [b"global_config_v2"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// Lottery's token account (source of refund)
    #[account(
        mut,
        constraint = lottery_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,

    /// Buyer's token account (destination for refund)
    #[account(
        mut,
        constraint = buyer_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount,
        constraint = buyer_token_account.owner == buyer.key() @ LotteryError::InvalidTokenAccount
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,

    #[account(mut)] // Buyer must sign to receive refund
    pub buyer: Signer<'info>,

    // Need the lottery account to act as signer for the transfer
    // The authority is the lottery account PDA itself.
    // We will use CPI seeds.

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<ClaimRefund>) -> Result<()> {
    let lottery_account = &ctx.accounts.lottery_account;
    let ticket_account = &mut ctx.accounts.ticket_account;
    let clock = Clock::get()?;
    
    let refund_amount = lottery_account.ticket_price;

    // Derive lottery PDA bump and handle lifetimes for seeds
    let lottery_type_string = lottery_account.lottery_type.to_string(); // Store string
    let draw_time_bytes = lottery_account.draw_time.to_le_bytes();
    let (_lottery_pda, lottery_bump) = Pubkey::find_program_address(
        &[
            b"lottery",
            lottery_type_string.as_bytes(), // Use stored string's bytes
            &draw_time_bytes
        ],
        ctx.program_id
    );
    
    let lottery_seeds = &[ 
        b"lottery".as_ref(),
        lottery_type_string.as_bytes(), // Use stored string's bytes
        draw_time_bytes.as_ref(),
        &[lottery_bump]
    ];
    let signer_seeds = &[&lottery_seeds[..]];

    // Transfer refund amount from lottery token account to buyer
    if refund_amount > 0 {
        let refund_transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.lottery_token_account.to_account_info(),
                to: ctx.accounts.buyer_token_account.to_account_info(),
                authority: lottery_account.to_account_info(), // Lottery PDA is the authority
            },
            signer_seeds, // Sign with derived lottery PDA seeds
        );
        token::transfer(refund_transfer_ctx, refund_amount)?;
        msg!("Transferred refund of {} to buyer", refund_amount);
    }

    // Mark ticket as claimed/refunded
    ticket_account.is_claimed = true;
    msg!("Marked ticket as refunded (claimed=true)");

    // Emit event
    emit!(TicketRefunded {
        lottery_id: lottery_account.key(),
        ticket_id: ticket_account.id,
        buyer: ctx.accounts.buyer.key(),
        refund_amount: refund_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
} 