use anchor_lang::prelude::*;
use crate::state::lottery::{LotteryAccount, LotteryType, LotteryState};
use crate::state::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::LotteryCreated;

#[derive(Accounts)]
#[instruction(lottery_type_enum: LotteryType, ticket_price: u64, draw_time: i64, target_prize_pool: u64, nonce: u64)]
pub struct CreateLottery<'info> {
    #[account(
        init,
        payer = creator,
        space = LotteryAccount::ACCOUNT_SIZE,
        seeds = [b"lottery", creator.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub lottery_account: Box<Account<'info, LotteryAccount>>,

    #[account(
        seeds = [b"global_config_v2"],
        bump,
        constraint = global_config.admin == creator.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Box<Account<'info, GlobalConfig>>,

    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

pub fn create_lottery_handler(
    ctx: Context<CreateLottery>,
    lottery_type_enum: LotteryType,
    ticket_price: u64,
    draw_time: i64,
    _target_prize_pool: u64,
    nonce: u64,
) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let _global_config = &ctx.accounts.global_config;
    let clock = Clock::get()?;

    // Validate inputs
    if ticket_price == 0 {
        return Err(LotteryError::InvalidTicketPrice.into());
    }
    
    if draw_time <= clock.unix_timestamp {
        return Err(LotteryError::InvalidDrawTime.into());
    }

    // Initialize Lottery Account
    lottery_account.lottery_type = lottery_type_enum.clone();
    lottery_account.ticket_price = ticket_price;
    lottery_account.draw_time = draw_time;
    lottery_account.prize_pool = 0;
    lottery_account.total_tickets = 0;
    lottery_account.winning_ticket = None;
    lottery_account.state = LotteryState::Created;
    lottery_account.authority = ctx.accounts.creator.key();
    lottery_account.last_ticket_id = 0;
    lottery_account.vrf_client = None;
    lottery_account.vrf_request_key = None;
    lottery_account.vrf_randomness = None;
    lottery_account.randomness_fulfilled = false;
    lottery_account.flags = 0; // All flags start as false
    lottery_account.created_at = clock.unix_timestamp;
    lottery_account.completed_at = None;
    lottery_account.nonce = nonce;

    emit!(LotteryCreated {
        lottery_id: lottery_account.key(),
        lottery_type: lottery_type_enum.to_string(),
        ticket_price,
        draw_time,
        target_prize_pool: 0, // No longer stored in account
    });

    Ok(())
}