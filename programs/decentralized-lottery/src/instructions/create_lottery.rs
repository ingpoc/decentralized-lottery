// src/instructions/create_lottery.rs
use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryType, LotteryState};
use crate::state::treasury::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::LotteryCreated;

#[derive(Accounts)]
#[instruction(
    lottery_type_enum: LotteryType,
    ticket_price: u64,
    draw_time: i64,
    prize_pool: u64,
)]
pub struct CreateLottery<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + 64 + 8 + 8 + 8 + 8 + (1 + 32) + 1 + 32 + 32,
        seeds = [
            b"lottery",
            lottery_type_enum.to_string().as_bytes(),
            &draw_time.to_le_bytes()
        ],
        bump
    )]
    pub lottery_account: Account<'info, LotteryAccount>,
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<CreateLottery>,
    lottery_type_enum: LotteryType,
    ticket_price: u64,
    draw_time: i64,
    prize_pool: u64,
) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let global_config = &ctx.accounts.global_config;

    // Validate inputs
    if ticket_price == 0 {
        return Err(LotteryError::InvalidTicketPrice.into());
    }
    if prize_pool == 0 {
        return Err(LotteryError::InvalidPrizePool.into());
    }
    if draw_time <= Clock::get().unwrap().unix_timestamp {
        return Err(LotteryError::InvalidDrawTime.into());
    }

    // Initialize Lottery Account
    lottery_account.lottery_type = lottery_type_enum.clone();
    lottery_account.ticket_price = ticket_price;
    lottery_account.draw_time = draw_time;
    lottery_account.prize_pool = prize_pool;
    lottery_account.total_tickets = 0;
    lottery_account.winning_numbers = None;
    lottery_account.state = LotteryState::Created;
    lottery_account.created_by = ctx.accounts.creator.key();
    lottery_account.global_config = global_config.key();

    emit!(LotteryCreated {
        lottery_id: lottery_account.key(),
        lottery_type: lottery_type_enum.to_string(),
        ticket_price,
        draw_time,
        prize_pool,
    });

    Ok(())
}