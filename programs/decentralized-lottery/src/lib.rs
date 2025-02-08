// src/lib.rs
use anchor_lang::prelude::*;

declare_id!("6RHVMDZ3MmHw224mMmqmMnXttgovREK57gD2NHgjTpz5");

pub mod instructions;
pub mod state;
pub mod utils;
pub mod errors;
pub mod events;

use instructions::create_lottery::*;
use state::lottery::*;

#[program]
pub mod decentralized_lottery {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
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
}

#[derive(Accounts)]
pub struct Initialize {}