use anchor_lang::prelude::*;
use anchor_spl::token::Token;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteType, RouletteState}
};
use crate::constants::*;
use crate::events::RouletteCreated;


#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init_if_needed,
        payer = authority,
        space = GlobalConfig::ACCOUNT_SIZE,
        seeds = [GLOBAL_CONFIG_SEED],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// First roulette game created automatically on initialization
    #[account(
        init_if_needed,
        payer = authority,
        space = RouletteAccount::ACCOUNT_SIZE,
        seeds = [ROULETTE_SEED, authority.key().as_ref(), &1u64.to_le_bytes()],
        bump
    )]
    pub first_roulette: Account<'info, RouletteAccount>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: USDC mint account
    pub usdc_mint: AccountInfo<'info>,
    
    /// CHECK: Treasury token account for collecting fees
    #[account(mut)]
    pub treasury_token_account: AccountInfo<'info>,
    
    /// First roulette token account (PDA) - will be created when needed
    /// CHECK: PDA for token account
    #[account(
        seeds = [ROULETTE_TOKEN_SEED, first_roulette.key().as_ref()],
        bump
    )]
    pub first_roulette_token_account: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let first_roulette = &mut ctx.accounts.first_roulette;
    let clock = Clock::get()?;
    
    // Always reinitialize global config for testing
    global_config.authority = ctx.accounts.authority.key();
    global_config.usdc_mint = ctx.accounts.usdc_mint.key();
    global_config.treasury_token_account = ctx.accounts.treasury_token_account.key();
    global_config.treasury_fee_percentage = DEFAULT_TREASURY_FEE_PERCENTAGE;
    global_config.is_paused = false;
    global_config.min_game_duration = DEFAULT_MIN_GAME_DURATION;
    global_config.max_game_duration = DEFAULT_MAX_GAME_DURATION;
    global_config.min_bet_amount = DEFAULT_MIN_BET_AMOUNT;
    global_config.max_bet_amount = DEFAULT_MAX_BET_AMOUNT;
    global_config.max_players_per_game = DEFAULT_MAX_PLAYERS_PER_GAME;
    global_config.created_at = clock.unix_timestamp;
    global_config.updated_at = clock.unix_timestamp;
    global_config.bump = ctx.bumps.global_config;
    
    // Always reinitialize first roulette game to start automation
    let nonce = 1u64;
    let roulette_type = RouletteType::European;
    let min_bet = global_config.min_bet_amount;
    let max_bet = global_config.max_bet_amount;
    let game_duration = global_config.min_game_duration;
    
    first_roulette.nonce = nonce;
    first_roulette.created_by = ctx.accounts.authority.key();
    first_roulette.authority = ctx.accounts.authority.key();
    first_roulette.global_config = global_config.key();
    first_roulette.roulette_type = roulette_type.clone();
    first_roulette.state = RouletteState::Open;
    first_roulette.min_bet = min_bet;
    first_roulette.max_bet = max_bet;
    first_roulette.game_duration = game_duration;
    first_roulette.start_time = clock.unix_timestamp;
    
    // Calculate game timing
    first_roulette.betting_duration = BETTING_DURATION;
    first_roulette.betting_end_time = clock.unix_timestamp + BETTING_DURATION;
    first_roulette.spin_time = clock.unix_timestamp + BETTING_DURATION + LOCK_DURATION;
    first_roulette.reveal_time = clock.unix_timestamp + BETTING_DURATION + REVEAL_DELAY;
    first_roulette.end_time = clock.unix_timestamp + game_duration;
    
    // Initialize counters
    first_roulette.total_bets = 0;
    first_roulette.total_bet_amount = 0;
    first_roulette.total_players = 0;
    first_roulette.winning_number = None;
    first_roulette.last_bet_id = 0;
    first_roulette.total_payouts = 0;
    first_roulette.house_edge_collected = 0;
    first_roulette.treasury_fee_collected = 0;
    first_roulette.created_at = clock.unix_timestamp;
    first_roulette.completed_at = None;
    first_roulette.is_settled = false;
    first_roulette.bump = ctx.bumps.first_roulette;
    
    // VRF fields
    first_roulette.vrf_client = None;
    first_roulette.vrf_randomness = None;
    first_roulette.vrf_request_key = None;
    first_roulette.randomness_fulfilled = false;
    
    emit!(RouletteCreated {
        roulette_id: first_roulette.key(),
        authority: ctx.accounts.authority.key(),
        roulette_type: roulette_type.clone(),
        min_bet,
        max_bet,
        game_duration,
        start_time: clock.unix_timestamp,
        betting_end_time: first_roulette.betting_end_time,
        spin_time: first_roulette.spin_time,
        end_time: first_roulette.end_time,
        is_autonomous: false,
        keeper: ctx.accounts.authority.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Roulette global config initialized and first game created: {}", first_roulette.key());
    
    Ok(())
}