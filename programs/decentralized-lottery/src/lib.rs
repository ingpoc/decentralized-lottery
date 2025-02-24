// src/lib.rs
use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

use instructions::create_lottery::*;
use instructions::buy_ticket::*;
use instructions::transition_state::*;
use instructions::cancel_lottery::*;
use state::lottery::*;
use state::treasury::GlobalConfig;

declare_id!("F1pffGp4n5qyNRcCnpoTH5CEfVKQEGxAxmRuRScUw4tz");

pub mod instructions;
pub mod state;
pub mod utils;
pub mod errors;
pub mod events;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 2 + 32 + 32,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod decentralized_lottery {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_config = &mut ctx.accounts.global_config;
        
        // Initialize with default values
        global_config.treasury = ctx.accounts.admin.key();
        global_config.treasury_fee_percentage = 250; // 2.5%
        global_config.admin = ctx.accounts.admin.key();
        global_config.usdc_mint = ctx.accounts.usdc_mint.key();
        
        Ok(())
    }

    pub fn create_lottery(
        ctx: Context<CreateLottery>,
        lottery_type_enum: LotteryType,
        ticket_price: u64,
        draw_time: i64,
        prize_pool: u64,
    ) -> Result<()> {
        instructions::create_lottery::handler(ctx, lottery_type_enum, ticket_price, draw_time, prize_pool)
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>, number_of_tickets: u64) -> Result<()> {
        instructions::buy_ticket::handler(ctx, number_of_tickets)
    }

    pub fn transition_state(ctx: Context<TransitionState>, next_state: LotteryState) -> Result<()> {
        instructions::transition_state::handler(ctx, next_state)
    }

    pub fn cancel_lottery(ctx: Context<CancelLottery>) -> Result<()> {
        instructions::cancel_lottery::handler(ctx)
    }
}