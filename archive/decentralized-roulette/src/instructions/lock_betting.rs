use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::BettingLocked;
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct LockBetting<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Open @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Anyone can call this instruction when the betting period ends
    pub caller: Signer<'info>,
}

pub fn handler(ctx: Context<LockBetting>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Validate timing - betting period must be over
    require!(
        clock.unix_timestamp >= roulette.betting_end_time,
        RouletteError::TooEarly
    );
    
    // Validate we haven't exceeded the spin time
    require!(
        clock.unix_timestamp < roulette.spin_time,
        RouletteError::TooLate
    );
    
    // Transition to locked state
    roulette.state = RouletteState::Locked;
    // roulette.updated_at = clock.unix_timestamp;
    
    // Emit betting locked event
    emit!(BettingLocked {
        roulette_id: roulette.key(),
        total_bets: roulette.total_bets,
        total_bet_amount: roulette.total_bet_amount,
        total_players: roulette.total_players,
        spin_time: roulette.spin_time,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Betting locked for roulette: {}, {} bets placed", 
         roulette.key(), roulette.total_bets);
    
    Ok(())
}