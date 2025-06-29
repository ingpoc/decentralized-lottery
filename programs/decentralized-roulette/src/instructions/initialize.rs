use anchor_lang::prelude::*;
use crate::state::global_config::GlobalConfig;
use crate::constants::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = GlobalConfig::ACCOUNT_SIZE,
        seeds = [GLOBAL_CONFIG_SEED],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: USDC mint account
    pub usdc_mint: AccountInfo<'info>,
    
    /// CHECK: Treasury token account for collecting fees
    #[account(mut)]
    pub treasury_token_account: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<Initialize>) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let clock = Clock::get()?;
    
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
    
    msg!("Roulette global config initialized");
    
    Ok(())
}