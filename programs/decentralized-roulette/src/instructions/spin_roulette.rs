use anchor_lang::prelude::*;
use switchboard_v2::VrfAccountData;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::{RouletteSpinStarted, RandomnessRequested};
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct SpinRoulette<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Locked @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// VRF account for random number generation
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    /// VRF authority (should be the roulette account)
    pub vrf_authority: Signer<'info>,
    
    /// Switchboard program
    /// CHECK: This is the Switchboard program ID
    pub switchboard_program: UncheckedAccount<'info>,
    
    /// Anyone can call this instruction when the spin time arrives
    pub caller: Signer<'info>,
}

pub fn handler(ctx: Context<SpinRoulette>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Validate timing - must be at or after spin time
    require!(
        clock.unix_timestamp >= roulette.spin_time,
        RouletteError::TooEarly
    );
    
    // Validate we haven't exceeded the reveal time
    require!(
        clock.unix_timestamp < roulette.reveal_time,
        RouletteError::TooLate
    );
    
    // Validate VRF authority is the roulette account
    let roulette_key = roulette.key();
    let roulette_seeds = &[
        ROULETTE_SEED,
        roulette.creator.as_ref(),
        &roulette.id.to_le_bytes(),
        &[roulette.bump],
    ];
    
    require!(
        ctx.accounts.vrf_authority.key() == roulette_key,
        RouletteError::InvalidAuthority
    );
    
    // Store VRF client reference
    roulette.vrf_client = Some(ctx.accounts.vrf.key());
    
    // Transition to spinning state
    roulette.state = RouletteState::Spinning;
    roulette.updated_at = clock.unix_timestamp;
    
    // Request randomness from VRF
    let vrf_request_cpi = CpiContext::new_with_signer(
        ctx.accounts.switchboard_program.to_account_info(),
        switchboard_v2::cpi::accounts::VrfRequestRandomness {
            vrf: ctx.accounts.vrf.to_account_info(),
            authority: ctx.accounts.vrf_authority.to_account_info(),
        },
        &[roulette_seeds],
    );
    
    switchboard_v2::cpi::vrf_request_randomness(vrf_request_cpi)?;
    
    // Emit events
    emit!(RouletteSpinStarted {
        roulette_id: roulette.key(),
        vrf_client: ctx.accounts.vrf.key(),
        vrf_request_key: ctx.accounts.vrf.key(), // In practice, this would be the request key
        timestamp: clock.unix_timestamp,
    });
    
    emit!(RandomnessRequested {
        roulette_id: roulette.key(),
        vrf_client: ctx.accounts.vrf.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Roulette spin initiated for: {}", roulette.key());
    
    Ok(())
}