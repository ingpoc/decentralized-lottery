use anchor_lang::prelude::*;

declare_id!("9SL8XkX3pvqZ2fjiLMhCFfQn7Gfmpd9ru8rtHFsAPVgq");

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use state::lottery::{LotteryType, LotteryState};
use instructions::*;

#[program]
pub mod decentralized_lottery {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn create_lottery(
        ctx: Context<CreateLottery>,
        lottery_type_enum: LotteryType,
        ticket_price: u64,
        draw_time: i64,
        target_prize_pool: u64,
        nonce: u64,
    ) -> Result<()> {
        instructions::create_lottery::handler(
            ctx,
            lottery_type_enum,
            ticket_price,
            draw_time,
            target_prize_pool,
            nonce,
        )
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        instructions::buy_ticket::handler(ctx)
    }

    pub fn transition_state(ctx: Context<TransitionState>, next_state: LotteryState) -> Result<()> {
        instructions::transition_state::handler(ctx, next_state)
    }

    pub fn select_winner(ctx: Context<SelectWinner>) -> Result<()> {
        instructions::select_winner::handler(ctx)
    }
            
    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        instructions::claim_prize::handler(ctx)
    }

    pub fn update_config(ctx: Context<UpdateConfig> /*, new_fee_percentage: Option<u16> */) -> Result<()> {
        instructions::update_config::handler(ctx /*, new_fee_percentage */)
    }

    pub fn settle_randomness(ctx: Context<SettleRandomness>) -> Result<()> {
        instructions::settle_randomness::handler(ctx)
    }

    // Add other instruction handlers as needed (e.g., for VRF)
}