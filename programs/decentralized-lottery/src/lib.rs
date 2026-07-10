use anchor_lang::prelude::*;

declare_id!("4WwZRDTd7ZajA3txfqnrER6EhCNwVKGVAczcZV3Vam58");

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
        instructions::initialize::initialize_handler(ctx)
    }

    pub fn create_lottery(
        ctx: Context<CreateLottery>,
        lottery_type_enum: LotteryType,
        ticket_price: u64,
        draw_time: i64,
        target_prize_pool: u64,
        nonce: u64,
    ) -> Result<()> {
        instructions::create_lottery::create_lottery_handler(
            ctx,
            lottery_type_enum,
            ticket_price,
            draw_time,
            target_prize_pool,
            nonce,
        )
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        instructions::buy_ticket::buy_ticket_handler(ctx)
    }

    pub fn transition_state(ctx: Context<TransitionState>, next_state: LotteryState) -> Result<()> {
        instructions::transition_state::transition_state_handler(ctx, next_state)
    }

    pub fn select_winner(ctx: Context<SelectWinner>) -> Result<()> {
        instructions::select_winner::select_winner_handler(ctx)
    }
            
    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        instructions::claim_prize::claim_prize_handler(ctx)
    }

    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_fee_percentage: Option<u16>,
        new_admin: Option<Pubkey>,
        is_paused: Option<bool>
    ) -> Result<()> {
        instructions::update_config::update_config_handler(ctx, new_fee_percentage, new_admin, is_paused)
    }

    pub fn settle_randomness(ctx: Context<SettleRandomness>) -> Result<()> {
        instructions::settle_randomness::settle_randomness_handler(ctx)
    }

    // === EMERGENCY SECURITY CONTROLS ===
    
    /// Emergency pause/unpause functionality
    /// Only callable by the global admin
    pub fn emergency_pause_toggle(ctx: Context<EmergencyPauseToggle>, pause: bool) -> Result<()> {
        instructions::emergency_pause::emergency_pause_toggle_handler(ctx, pause)
    }

    /// Force cancel a lottery in emergency situations
    /// Only callable by the global admin
    pub fn force_cancel_lottery(ctx: Context<ForceCancelLottery>, reason: String) -> Result<()> {
        instructions::emergency_pause::force_cancel_lottery_handler(ctx, reason)
    }

    // Add other instruction handlers as needed (e.g., for VRF)
}