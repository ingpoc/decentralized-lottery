// src/lib.rs
use anchor_lang::prelude::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount};

use instructions::*;
pub use instructions::{
    CreateLottery, BuyTicket, TransitionState, CancelLottery, SettleRandomness, ClaimPrize, ClaimRefund,
    // Add new VRF instruction contexts
    InitVrfClient, RequestRandomness, ConsumeRandomness
};
pub use state::{
    LotteryType, LotteryState, GlobalConfig, LotteryAccount, TicketAccount,
    // Export VRF-related state
    vrf::VrfClientState
};

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
        space = GlobalConfig::ACCOUNT_SIZE,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(
        constraint = treasury_token_account.mint == usdc_mint.key() @ LotteryError::InvalidTokenAccount
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"global_config"],
        bump,
        has_one = admin @ LotteryError::UnauthorizedAccess
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
}

#[program]
pub mod decentralized_lottery {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_config = &mut ctx.accounts.global_config;
        global_config.admin = ctx.accounts.admin.key();
        global_config.treasury_fee_percentage = 250; // Default 2.5%
        global_config.usdc_mint = ctx.accounts.usdc_mint.key();
        global_config.treasury_token_account = ctx.accounts.treasury_token_account.key();
        Ok(())
    }

    pub fn update_config(ctx: Context<UpdateConfig>) -> Result<()> {
        let global_config = &mut ctx.accounts.global_config;
        global_config.usdc_mint = ctx.accounts.usdc_mint.key();
        msg!("Updated USDC mint address to: {}", global_config.usdc_mint);
        Ok(())
    }

    pub fn create_lottery(
        ctx: Context<CreateLottery>,
        lottery_type_enum: LotteryType,
        ticket_price: u64,
        draw_time: i64,
        target_prize_pool: u64,
    ) -> Result<()> {
        instructions::create_lottery::handler(ctx, lottery_type_enum, ticket_price, draw_time, target_prize_pool)
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        instructions::buy_ticket::handler(ctx)
    }

    pub fn transition_state(ctx: Context<TransitionState>, next_state: LotteryState) -> Result<()> {
        instructions::transition_state::handler(ctx, next_state)
    }

    pub fn settle_randomness(ctx: Context<SettleRandomness>) -> Result<()> {
        instructions::settle_randomness::handler(ctx)
    }

    pub fn cancel_lottery(ctx: Context<CancelLottery>) -> Result<()> {
        instructions::cancel_lottery::handler(ctx)
    }

    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        instructions::claim_prize::handler(ctx)
    }

    pub fn claim_refund(ctx: Context<ClaimRefund>) -> Result<()> {
        instructions::claim_refund::handler(ctx)
    }
    
    /// Initialize a VRF client account for a lottery
    /// This creates the necessary state for interacting with Switchboard VRF
    pub fn init_vrf_client(ctx: Context<InitVrfClient>) -> Result<()> {
        instructions::vrf::init_vrf_client_handler(ctx)
    }

    /// Request randomness from Switchboard VRF for a lottery draw
    /// This should be called when a lottery is ready to be drawn
    pub fn request_randomness(ctx: Context<RequestRandomness>) -> Result<()> {
        instructions::vrf::request_randomness_handler(ctx)
    }

    /// Consume the randomness produced by Switchboard VRF
    /// This verifies and extracts the random value for lottery winner selection
    pub fn consume_randomness(ctx: Context<ConsumeRandomness>) -> Result<()> {
        instructions::vrf::consume_randomness_handler(ctx)
    }
}
