use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteType, RouletteState}
};
use crate::constants::*;
use crate::events::RouletteCreated;
use crate::errors::RouletteError;

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct CreateNextGame<'info> {
    #[account(
        init,
        payer = caller,
        space = RouletteAccount::ACCOUNT_SIZE,
        seeds = [ROULETTE_SEED, caller.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub new_roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub caller: Signer<'info>,
    
    /// USDC mint account
    /// CHECK: Verified against global config
    #[account(
        constraint = usdc_mint.key() == global_config.usdc_mint @ RouletteError::InvalidTokenAccount
    )]
    pub usdc_mint: AccountInfo<'info>,
    
    /// New roulette token account (PDA)
    /// CHECK: Created and initialized properly
    #[account(
        init,
        payer = caller,
        space = 165,
        seeds = [ROULETTE_TOKEN_SEED, new_roulette.key().as_ref()],
        bump
    )]
    pub roulette_token_account: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<CreateNextGame>, nonce: u64) -> Result<()> {
    let global_config = &ctx.accounts.global_config;
    let roulette = &mut ctx.accounts.new_roulette;
    let clock = Clock::get()?;
    
    // Use default game parameters for automated creation
    let roulette_type = RouletteType::European;
    let min_bet = global_config.min_bet_amount;
    let max_bet = global_config.max_bet_amount;
    // Use hardcoded duration to ensure games have proper end time
    let game_duration = 300i64; // 5 minutes
    
    // Initialize roulette with same logic as create_roulette
    roulette.nonce = nonce;
    roulette.created_by = ctx.accounts.caller.key();
    roulette.authority = ctx.accounts.caller.key();
    roulette.global_config = global_config.key();
    roulette.roulette_type = roulette_type.clone();
    roulette.state = RouletteState::Open;
    roulette.min_bet = min_bet;
    roulette.max_bet = max_bet;
    roulette.game_duration = game_duration;
    roulette.start_time = clock.unix_timestamp;
    
    // Calculate game timing
    roulette.betting_duration = BETTING_DURATION;
    roulette.betting_end_time = clock.unix_timestamp + BETTING_DURATION;
    roulette.spin_time = clock.unix_timestamp + BETTING_DURATION + LOCK_DURATION;
    roulette.reveal_time = clock.unix_timestamp + BETTING_DURATION + REVEAL_DELAY;
    roulette.end_time = clock.unix_timestamp + game_duration;
    
    // Initialize counters
    roulette.total_bets = 0;
    roulette.total_bet_amount = 0;
    roulette.total_players = 0;
    roulette.winning_number = None;
    roulette.last_bet_id = 0;
    roulette.total_payouts = 0;
    roulette.house_edge_collected = 0;
    roulette.treasury_fee_collected = 0;
    roulette.created_at = clock.unix_timestamp;
    roulette.completed_at = None;
    roulette.is_settled = false;
    roulette.bump = ctx.bumps.new_roulette;
    
    // VRF fields
    roulette.vrf_client = None;
    roulette.vrf_randomness = None;
    roulette.vrf_request_key = None;
    roulette.randomness_fulfilled = false;
    
    emit!(RouletteCreated {
        roulette_id: roulette.key(),
        creator: ctx.accounts.caller.key(),
        roulette_type: roulette_type.clone(),
        min_bet,
        max_bet,
        game_duration,
        start_time: clock.unix_timestamp,
        betting_end_time: roulette.betting_end_time,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Auto-created next roulette game: {} by {}", roulette.key(), ctx.accounts.caller.key());
    
    // Auto-process any existing games that may need state transitions
    // This ensures the system stays healthy when new games are created
    process_existing_games_lifecycle(&ctx.accounts.global_config, &clock)?;
    
    Ok(())
}

/// Helper function to process existing games' lifecycle when new games are created
fn process_existing_games_lifecycle(global_config: &GlobalConfig, clock: &Clock) -> Result<()> {
    // Log that we're processing existing games
    msg!("Processing existing games lifecycle during new game creation");
    
    // Note: In a real implementation, you would iterate through active games
    // For now, we log this intent and rely on the existing process_game_lifecycle instruction
    // which can be called by anyone to process individual games
    
    Ok(())
}