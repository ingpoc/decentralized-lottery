use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
// use tuktuk_sdk::cpi::{queue_task, QueueTaskV0}; // Temporarily disabled due to version conflict
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState, RouletteType},
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
        seeds = [ROULETTE_SEED, creator.key().as_ref(), nonce.to_le_bytes().as_ref()],
        bump
    )]
    pub roulette: Account<'info, RouletteAccount>,

    #[account(
        init_if_needed,
        payer = creator,
        associated_token::mint = usdc_mint,
        associated_token::authority = roulette,
    )]
    pub roulette_usdc_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,

    pub usdc_mint: Account<'info, Mint>,

    #[account(mut)]
    pub creator: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,

    // TukTuk accounts - temporarily disabled due to version conflict
    // #[account(mut)]
    // pub tuktuk_queue: AccountInfo<'info>,  // Queue PDA
    // pub tuktuk_program: Program<'info, tuktuk_sdk::TuktukProgram>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CreateRouletteParams {
    pub roulette_type: RouletteType,
    pub min_bet: u64,
    pub max_bet: u64,
    pub game_duration: i64,
    pub betting_duration: i64,
    pub nonce: u64,
}

pub fn handler(
    ctx: Context<CreateRoulette>, 
    roulette_type: RouletteType,
    min_bet: u64,
    max_bet: u64,
    game_duration: i64,
    nonce: u64
) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    let roulette = &mut ctx.accounts.roulette;
    roulette.roulette_type = roulette_type;
    roulette.min_bet = min_bet;
    roulette.max_bet = max_bet;
    roulette.game_duration = game_duration;
    roulette.betting_duration = BETTING_DURATION; // 3 minutes default
    roulette.start_time = current_time;
    roulette.betting_end_time = current_time + BETTING_DURATION;
    roulette.spin_time = roulette.betting_end_time + 30;
    roulette.reveal_time = roulette.spin_time + 30;
    roulette.end_time = current_time + game_duration;
    roulette.state = RouletteState::Open;
    roulette.created_by = ctx.accounts.creator.key();
    roulette.authority = ctx.accounts.global_config.authority;
    roulette.global_config = ctx.accounts.global_config.key();
    roulette.roulette_usdc_account = ctx.accounts.roulette_usdc_account.key();
    roulette.created_at = current_time;
    roulette.nonce = nonce;
    roulette.bump = ctx.bumps.roulette;

    // Automation will be handled by external crank calls to public_lifecycle_keeper
    // This provides the same functionality without requiring TukTuk dependency compatibility

    // Emit creation event
    emit!(RouletteCreated {
        roulette_id: roulette.key(),
        creator: ctx.accounts.creator.key(),
        roulette_type: roulette_type,
        min_bet: min_bet,
        max_bet: max_bet,
        game_duration: game_duration,
        start_time: current_time,
        betting_end_time: roulette.betting_end_time,
        timestamp: current_time,
    });

    msg!("Created roulette: {} (automation via public crank)", 
         roulette.key());

    Ok(())
}