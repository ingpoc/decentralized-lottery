use anchor_lang::prelude::*;

declare_id!("CWiMTzfG7e3Jdta6sSsm9xVZjiA1ZHdtwuYjpQyjrW4F");

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

    // VRF-related functions for production-ready verifiable randomness
    pub fn spin_roulette(ctx: Context<SpinRoulette>) -> Result<()> {
        instructions::spin_roulette::handler(ctx)
    }

    pub fn settle_randomness(ctx: Context<SettleRandomness>) -> Result<()> {
        instructions::settle_randomness::handler(ctx)
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        instructions::claim_winnings::handler(ctx)
    }

    pub fn cancel_roulette(ctx: Context<CancelRoulette>, reason: String) -> Result<()> {
        instructions::cancel_roulette::handler(ctx, instructions::cancel_roulette::CancelRouletteArgs { reason })
    }

    // Automation instructions for continuous game management
    pub fn process_game_lifecycle(ctx: Context<ProcessGameLifecycle>) -> Result<()> {
        instructions::process_game_lifecycle::handler(ctx)
    }

    pub fn create_next_game(ctx: Context<CreateNextGame>, nonce: u64) -> Result<()> {
        instructions::create_next_game::handler(ctx, nonce)
    }

    pub fn process_automation(ctx: Context<ProcessAutomation>) -> Result<()> {
        instructions::process_automation::handler(ctx)
    }

    // TukTuk-based automatic lifecycle processing (crank-turner calls this)
    pub fn public_lifecycle_keeper(ctx: Context<PublicLifecycleKeeper>) -> Result<()> {
        instructions::public_lifecycle_keeper::handler(ctx)
    }

    // === EMERGENCY SECURITY CONTROLS ===
    
    /// Emergency pause/unpause functionality
    /// Only callable by the global authority
    pub fn emergency_pause_toggle(ctx: Context<EmergencyPauseToggle>, pause: bool) -> Result<()> {
        instructions::emergency_pause::handler(ctx, pause)
    }

    /// Force end a game in emergency situations
    /// Only callable by the global authority
    pub fn force_end_game(ctx: Context<ForceEndGame>) -> Result<()> {
        instructions::emergency_pause::force_end_game_handler(ctx)
    }
}