// src/instructions/create_lottery.rs
use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Mint};
use crate::state::lottery::{LotteryAccount, LotteryType, LotteryState};
use crate::state::treasury::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::LotteryCreated;

#[derive(Accounts)]
#[instruction(
    lottery_type_enum: LotteryType,
    ticket_price: u64,
    draw_time: i64,
    target_prize_pool: u64,
)]
pub struct CreateLottery<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + 1 + 8 + 8 + 8 + 8 + 33 + 1 + 32 + 32 + 1 + 8 + 33 + 33 + 1 + 8 + 1, // ~226 bytes
        seeds = [
            b"lottery",
            lottery_type_enum.to_string().as_bytes(),
            &draw_time.to_le_bytes()
        ],
        bump
    )]
    /// The lottery account storing all metadata about the lottery.
    /// PDA Derivation Explanation:
    /// - Seed prefix: b"lottery" - A static identifier for lottery accounts.
    /// - Seed 1: lottery_type_enum.to_string().as_bytes() - The type of lottery (e.g., Daily, Weekly) as a string, ensuring uniqueness across different lottery types.
    /// - Seed 2: draw_time.to_le_bytes() - The scheduled draw time as little-endian bytes, ensuring uniqueness for lotteries of the same type but different draw times.
    /// - Bump: Automatically determined by Anchor to find a valid Program Derived Address (PDA) that can be signed by the program.
    /// Rationale: This combination ensures that each lottery is uniquely identifiable by its type and draw time, preventing collisions and allowing multiple lotteries of the same type to exist with different draw schedules.
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        seeds = [b"global_config"],
        bump,
        constraint = global_config.admin == creator.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Account<'info, GlobalConfig>,

    /// The mint for the token being used (USDC)
    #[account(
        constraint = token_mint.key() == global_config.usdc_mint @ LotteryError::InvalidTokenAccount
    )]
    pub token_mint: Account<'info, Mint>,

    /// The creator's token account (no longer needed for funding, but kept for consistency)
    #[account(
        constraint = creator_token_account.mint == token_mint.key() @ LotteryError::InvalidTokenAccount,
        constraint = creator_token_account.owner == creator.key() @ LotteryError::InvalidAccountOwner
    )]
    pub creator_token_account: Account<'info, TokenAccount>,

    /// The lottery's token account for prize pool
    #[account(
        init,
        payer = creator,
        seeds = [
            b"lottery_token",
            lottery_account.key().as_ref()
        ],
        bump,
        token::mint = token_mint,
        token::authority = lottery_account
    )]
    /// The token account holding the prize pool for this lottery.
    /// PDA Derivation Explanation:
    /// - Seed prefix: b"lottery_token" - A static identifier for lottery token accounts.
    /// - Seed 1: lottery_account.key().as_ref() - The public key of the associated lottery account, linking this token account uniquely to a specific lottery.
    /// - Bump: Automatically determined by Anchor to find a valid PDA.
    /// Rationale: Associating the token account with the lottery account's key ensures that each lottery has exactly one prize pool token account, preventing unauthorized access or confusion between different lotteries' funds.
    pub lottery_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<CreateLottery>,
    lottery_type_enum: LotteryType,
    ticket_price: u64,
    draw_time: i64,
    target_prize_pool: u64,
) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let global_config = &ctx.accounts.global_config;

    // Validate inputs
    if ticket_price == 0 {
        return Err(LotteryError::InvalidTicketPrice.into());
    }
    
    // Target prize pool can be zero, but if specified, it must be reasonable
    if target_prize_pool > 1_000_000_000_000 { // 1 million USDC (with 6 decimals)
        return Err(LotteryError::InvalidPrizePool.into());
    }
    
    if draw_time <= Clock::get()?.unix_timestamp {
        return Err(LotteryError::InvalidDrawTime.into());
    }

    // Initialize Lottery Account
    /// Lottery State Flow Explanation:
    /// - Initial State: Created - The lottery is initialized in the 'Created' state, indicating it is ready to accept ticket purchases.
    /// - Transition: From 'Created', the lottery can move to 'Active' (via a separate instruction or automatically based on conditions), then to 'AwaitingRandomness' when the draw time is reached, 'Completed' once randomness is settled and a winner is selected, or 'Expired' if no tickets are sold by the draw time.
    /// - Purpose: This state machine ensures a clear lifecycle for the lottery, preventing invalid operations (e.g., buying tickets after the draw) and providing transparency to participants.
    lottery_account.lottery_type = lottery_type_enum.clone();
    lottery_account.ticket_price = ticket_price;
    lottery_account.draw_time = draw_time;
    lottery_account.prize_pool = 0;
    lottery_account.target_prize_pool = target_prize_pool;
    lottery_account.total_tickets = 0;
    lottery_account.winning_ticket = None;
    lottery_account.state = LotteryState::Created;
    lottery_account.created_by = ctx.accounts.creator.key();
    lottery_account.global_config = global_config.key();
    lottery_account.auto_transition = false;
    lottery_account.last_ticket_id = 0;
    lottery_account.oracle_pubkey = None;
    lottery_account.vrf_request_account = None;
    lottery_account.is_prize_pool_locked = false;
    lottery_account.is_claimed = false;

    // No token transfer needed - prize pool starts at zero and builds from ticket sales

    emit!(LotteryCreated {
        lottery_id: lottery_account.key(),
        lottery_type: lottery_type_enum.to_string(),
        ticket_price,
        draw_time,
        target_prize_pool, // Changed from prize_pool to target_prize_pool
    });

    Ok(())
}