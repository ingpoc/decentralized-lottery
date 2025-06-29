use anchor_lang::prelude::*;

declare_id!("4ZVg5wU59Tr6pKAfxkTFsF2cffGrVRM2xqt1WbPUJrUB");

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;
use state::roulette::{RouletteType, BetType};

#[program]
pub mod decentralized_roulette {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handler(ctx)
    }

    pub fn create_roulette(
        ctx: Context<CreateRoulette>,
        roulette_type: RouletteType,
        min_bet: u64,
        max_bet: u64,
        game_duration: i64,
        nonce: u64,
    ) -> Result<()> {
        instructions::create_roulette::handler(
            ctx, roulette_type, min_bet, max_bet, game_duration, nonce
        )
    }

    pub fn place_bet(
        ctx: Context<PlaceBet>,
        bet_type: BetType,
        bet_amount: u64,
        bet_numbers: Vec<u8>,
    ) -> Result<()> {
        instructions::place_bet::handler(ctx, bet_type, bet_amount, bet_numbers)
    }

    pub fn lock_betting(ctx: Context<LockBetting>) -> Result<()> {
        instructions::lock_betting::handler(ctx)
    }

    // VRF-related functions temporarily commented out
    // pub fn spin_roulette(ctx: Context<SpinRoulette>) -> Result<()> {
    //     instructions::spin_roulette::handler(ctx)
    // }

    // pub fn settle_randomness(ctx: Context<SettleRandomness>) -> Result<()> {
    //     instructions::settle_randomness::handler(ctx)
    // }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        instructions::claim_winnings::handler(ctx)
    }

    pub fn cancel_roulette(ctx: Context<CancelRoulette>, reason: String) -> Result<()> {
        instructions::cancel_roulette::handler(ctx, instructions::cancel_roulette::CancelRouletteArgs { reason })
    }
}