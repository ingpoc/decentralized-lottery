use anchor_lang::prelude::*;
use crate::state::global_config::GlobalConfig;
use crate::constants::*;
use crate::errors::RouletteError;
use crate::events::EmergencyPause;

#[derive(Accounts)]
pub struct EmergencyPauseToggle<'info> {
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = global_config.authority == authority.key() @ RouletteError::InvalidAuthority
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Only the global authority can pause/unpause
    pub authority: Signer<'info>,
}

/// Emergency pause/unpause functionality
/// Only callable by the global authority
pub fn handler(ctx: Context<EmergencyPauseToggle>, pause: bool) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let clock = Clock::get()?;
    
    // Update pause state
    let previous_state = global_config.is_paused;
    global_config.is_paused = pause;
    global_config.updated_at = clock.unix_timestamp;
    
    // Emit event for transparency
    emit!(EmergencyPause {
        authority: ctx.accounts.authority.key(),
        previous_state,
        new_state: pause,
        timestamp: clock.unix_timestamp,
    });
    
    if pause {
        msg!("🚨 EMERGENCY PAUSE ACTIVATED by {}", ctx.accounts.authority.key());
    } else {
        msg!("✅ EMERGENCY PAUSE DEACTIVATED by {}", ctx.accounts.authority.key());
    }
    
    Ok(())
}

#[derive(Accounts)]
pub struct ForceEndGame<'info> {
    #[account(
        mut,
        constraint = roulette.state != crate::state::roulette::RouletteState::Completed @ RouletteError::InvalidGameState,
        constraint = roulette.state != crate::state::roulette::RouletteState::Cancelled @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, crate::state::roulette::RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = global_config.authority == authority.key() @ RouletteError::InvalidAuthority
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Only the global authority can force end games
    pub authority: Signer<'info>,
}

/// Force end a game in emergency situations
/// Only callable by the global authority
pub fn force_end_game_handler(ctx: Context<ForceEndGame>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    let old_state = roulette.state.clone();
    roulette.state = crate::state::roulette::RouletteState::Cancelled;
    roulette.completed_at = Some(clock.unix_timestamp);
    
    // Emit state change event
    emit!(crate::events::RouletteStateChanged {
        roulette_id: roulette.key(),
        old_state,
        new_state: crate::state::roulette::RouletteState::Cancelled,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("🚨 GAME FORCE ENDED by authority: {}", roulette.key());
    
    Ok(())
}
