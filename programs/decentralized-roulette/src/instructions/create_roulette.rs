use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteType, RouletteState}
};
use crate::constants::*;
use crate::events::RouletteCreated;
use crate::errors::RouletteError;

#[derive(Accounts)]
#[instruction(roulette_type: RouletteType, min_bet: u64, max_bet: u64, game_duration: i64, nonce: u64)]
pub struct CreateRoulette<'info> {
    #[account(
        init,  
        payer = creator,
        space = RouletteAccount::ACCOUNT_SIZE,
        seeds = [ROULETTE_SEED, creator.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub creator: Signer<'info>,
    
    /// CHECK: USDC mint account
    #[account(
        constraint = usdc_mint.key() == global_config.usdc_mint @ RouletteError::InvalidTokenAccount
    )]
    pub usdc_mint: AccountInfo<'info>,
    
    /// CHECK: Creator's USDC token account
    #[account(mut)]
    pub creator_token_account: AccountInfo<'info>,
    
    /// CHECK: Roulette token account (holds all bets)
    #[account(
        init,
        payer = creator,
        space = 165,
        seeds = [ROULETTE_TOKEN_SEED, roulette.key().as_ref()],
        bump
    )]
    pub roulette_token_account: AccountInfo<'info>,
    
    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<CreateRoulette>,
    roulette_type: RouletteType,
    min_bet: u64,
    max_bet: u64,
    game_duration: i64,
    nonce: u64,
) -> Result<()> {
    let global_config = &ctx.accounts.global_config;
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Validate game parameters
    require!(
        game_duration >= global_config.min_game_duration,
        RouletteError::GameDurationTooShort
    );
    require!(
        game_duration <= global_config.max_game_duration,
        RouletteError::GameDurationTooLong
    );
    require!(
        min_bet >= global_config.min_bet_amount,
        RouletteError::BetBelowMinimum
    );
    require!(
        max_bet <= global_config.max_bet_amount,
        RouletteError::BetExceedsMaximum
    );
    require!(min_bet <= max_bet, RouletteError::InvalidBetNumbers);
    
    // Initialize roulette account
    roulette.nonce = nonce;
    roulette.created_by = ctx.accounts.creator.key();
    roulette.authority = ctx.accounts.creator.key();
    roulette.global_config = ctx.accounts.global_config.key();
    roulette.roulette_type = roulette_type.clone();
    roulette.state = RouletteState::Open;
    roulette.min_bet = min_bet;
    roulette.max_bet = max_bet;
    roulette.game_duration = game_duration;
    roulette.start_time = clock.unix_timestamp;
    
    // Calculate game timing based on constants
    roulette.betting_duration = BETTING_DURATION;
    roulette.betting_end_time = clock.unix_timestamp + BETTING_DURATION;
    roulette.spin_time = clock.unix_timestamp + BETTING_DURATION + LOCK_DURATION;
    roulette.reveal_time = clock.unix_timestamp + BETTING_DURATION + REVEAL_DELAY;
    roulette.end_time = clock.unix_timestamp + game_duration;
    
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
    roulette.bump = ctx.bumps.roulette;
    
    // VRF fields (will be set when spinning)
    roulette.vrf_client = None;
    roulette.vrf_randomness = None;
    roulette.vrf_request_key = None;
    roulette.randomness_fulfilled = false;
    
    // Emit roulette created event
    emit!(RouletteCreated {
        roulette_id: roulette.key(),
        creator: ctx.accounts.creator.key(),
        roulette_type,
        min_bet,
        max_bet,
        game_duration,
        start_time: clock.unix_timestamp,
        betting_end_time: roulette.betting_end_time,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Roulette created with ID: {}", roulette.key());
    
    Ok(())
}