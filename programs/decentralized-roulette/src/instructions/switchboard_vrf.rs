use anchor_lang::prelude::*;
use switchboard_solana::{VrfAccountData, VrfRequestRandomness, SWITCHBOARD_PROGRAM_ID};
use crate::state::global_config::GlobalConfig;
use crate::state::roulette::{RouletteAccount, RouletteState};
use crate::errors::RouletteError;
use crate::events::{RandomnessRequested, RouletteStateChanged, VrfCompleted};
use crate::constants::*;

/// Switchboard VRF integration for secure randomness
/// This module provides production-ready Switchboard VRF integration
/// with proper fallback mechanisms for dev/test environments

#[derive(Accounts)]
pub struct InitializeSwitchboardVrf<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Created @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = global_config.authority == authority.key() @ RouletteError::InvalidAuthority
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Switchboard VRF account
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Switchboard program
    #[account(address = SWITCHBOARD_PROGRAM_ID)]
    pub switchboard_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RequestSwitchboardRandomness<'info> {
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
    
    /// Switchboard VRF account
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    /// Switchboard Oracle Queue account
    /// CHECK: Validated by Switchboard CPI
    #[account(mut)]
    pub oracle_queue: AccountInfo<'info>,
    
    /// Queue Authority
    /// CHECK: Validated by Switchboard CPI  
    #[account(mut)]
    pub queue_authority: AccountInfo<'info>,
    
    /// Data Buffer
    /// CHECK: Validated by Switchboard CPI
    #[account(mut)]
    pub data_buffer: AccountInfo<'info>,
    
    /// Permission account
    /// CHECK: Validated by Switchboard CPI
    #[account(mut)]
    pub permission: AccountInfo<'info>,
    
    /// Escrow account (for payment)
    /// CHECK: Validated by Switchboard CPI
    #[account(mut)]
    pub escrow: AccountInfo<'info>,
    
    /// Payer wallet for VRF fees
    /// CHECK: Validated by Switchboard CPI
    #[account(mut)]
    pub payer_wallet: AccountInfo<'info>,
    
    /// Payer authority
    /// CHECK: Validated by Switchboard CPI
    #[account(mut)]
    pub payer_authority: AccountInfo<'info>,
    
    /// Program state account (PDA authority for VRF)
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump
    )]
    pub program_state: Account<'info, GlobalConfig>,
    
    /// Recent blockhashes sysvar for entropy
    /// CHECK: Solana sysvar for recent blockhashes
    #[account(address = anchor_lang::solana_program::sysvar::recent_blockhashes::id())]
    pub recent_blockhashes: AccountInfo<'info>,
    
    /// CHECK: Switchboard program
    #[account(address = SWITCHBOARD_PROGRAM_ID)]
    pub switchboard_program: AccountInfo<'info>,
    
    /// Token program for escrow payment
    pub token_program: Program<'info, anchor_spl::token::Token>,
    
    pub caller: Signer<'info>,
}

#[derive(Accounts)]
pub struct ConsumeSwitchboardRandomness<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::AwaitingRandomness @ RouletteError::InvalidGameState,
        constraint = !roulette.randomness_fulfilled @ RouletteError::RandomnessAlreadyFulfilled
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    /// Switchboard VRF account containing the result
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    pub caller: Signer<'info>,
}

/// Initialize Switchboard VRF for a roulette game
pub fn initialize_switchboard_vrf_handler(ctx: Context<InitializeSwitchboardVrf>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Verify VRF account is owned by Switchboard
    require!(
        ctx.accounts.vrf.to_account_info().owner == &SWITCHBOARD_PROGRAM_ID,
        RouletteError::InvalidVrfAccount
    );
    
    // Store VRF account reference
    roulette.switchboard_vrf = Some(ctx.accounts.vrf.key());
    
    msg!("Switchboard VRF initialized for roulette: {}", roulette.key());
    Ok(())
}

/// Request randomness using Switchboard VRF
pub fn request_switchboard_randomness_handler(ctx: Context<RequestSwitchboardRandomness>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Validate timing
    require!(
        clock.unix_timestamp >= roulette.spin_time,
        RouletteError::TooEarly
    );
    
    require!(
        clock.unix_timestamp < roulette.reveal_time,
        RouletteError::TooLate
    );
    
    // Transition roulette state
    let old_state = roulette.state.clone();
    roulette.state = RouletteState::AwaitingRandomness;
    
    // Prepare Switchboard VRF request
    let global_config = &ctx.accounts.global_config;
    let state_seeds: &[&[u8]] = &[
        GLOBAL_CONFIG_SEED,
        &[global_config.bump],
    ];
    
    // Create VRF request using Switchboard CPI
    let vrf_request_randomness = VrfRequestRandomness {
        authority: ctx.accounts.program_state.to_account_info(),
        vrf: ctx.accounts.vrf.to_account_info(),
        oracle_queue: ctx.accounts.oracle_queue.to_account_info(),
        queue_authority: ctx.accounts.queue_authority.to_account_info(),
        data_buffer: ctx.accounts.data_buffer.to_account_info(),
        permission: ctx.accounts.permission.to_account_info(),
        escrow: ctx.accounts.escrow.clone(),
        payer_wallet: ctx.accounts.payer_wallet.clone(),
        payer_authority: ctx.accounts.payer_authority.to_account_info(),
        recent_blockhashes: ctx.accounts.recent_blockhashes.to_account_info(),
        program_state: ctx.accounts.program_state.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
    };
    
    msg!("Requesting Switchboard VRF randomness for roulette: {}", roulette.key());
    
    // Invoke the Switchboard VRF request with PDA authority
    vrf_request_randomness.invoke_signed(
        ctx.accounts.switchboard_program.to_account_info(),
        &[state_seeds],
    )?;
    
    // Store request timestamp for validation
    roulette.vrf_request_timestamp = Some(clock.unix_timestamp);
    
    // Emit events
    emit!(RouletteStateChanged {
        roulette_id: roulette.key(),
        old_state,
        new_state: RouletteState::AwaitingRandomness,
        timestamp: clock.unix_timestamp,
    });
    
    emit!(RandomnessRequested {
        roulette_id: roulette.key(),
        vrf_client: ctx.accounts.vrf.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Switchboard VRF request submitted successfully");
    Ok(())
}

/// Consume randomness from Switchboard VRF result
pub fn consume_switchboard_randomness_handler(ctx: Context<ConsumeSwitchboardRandomness>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Load VRF account data
    let vrf = ctx.accounts.vrf.load()?;
    
    // Validate minimum time has passed since request (prevents manipulation)
    if let Some(request_time) = roulette.vrf_request_timestamp {
        require!(
            clock.unix_timestamp >= request_time + VRF_REQUEST_DELAY,
            RouletteError::TooEarly
        );
    }
    
    // Get VRF result
    let result_buffer = vrf.get_result()?;
    if result_buffer.is_empty() {
        return Err(RouletteError::RandomnessNotReady.into());
    }
    
    // Extract randomness from Switchboard VRF result
    let randomness = extract_switchboard_randomness(&result_buffer)?;
    
    // Extract winning number based on roulette type
    let winning_number = extract_winning_number(&randomness, roulette.roulette_type);
    
    // Update roulette state
    roulette.vrf_randomness = Some(randomness);
    roulette.winning_number = Some(winning_number);
    roulette.randomness_fulfilled = true;
    roulette.state = RouletteState::Completed;
    roulette.completed_at = Some(clock.unix_timestamp);
    
    // Emit completion event
    emit!(VrfCompleted {
        roulette_id: roulette.key(),
        winning_number,
        randomness_source: "switchboard".to_string(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Switchboard VRF consumed. Winning number: {} for roulette: {}", winning_number, roulette.key());
    Ok(())
}

/// Extract randomness from Switchboard VRF result buffer
fn extract_switchboard_randomness(result_buffer: &[u8]) -> Result<[u8; 32]> {
    // Switchboard VRF returns 128-bit randomness
    // We need to extract and expand it to 256-bit for our use
    let value: &[u128] = bytemuck::cast_slice(result_buffer);
    
    if value.is_empty() {
        return Err(RouletteError::InvalidRandomness.into());
    }
    
    let vrf_result = value[0];
    
    // Convert to bytes and create secure 256-bit randomness
    let mut randomness = [0u8; 32];
    
    // Use the VRF result as primary entropy
    let vrf_bytes = vrf_result.to_le_bytes();
    randomness[..16].copy_from_slice(&vrf_bytes);
    
    // Add secondary entropy from clock and other sources for full 256-bit
    let clock = Clock::get().map_err(|_| RouletteError::ClockError)?;
    let timestamp_bytes = clock.unix_timestamp.to_le_bytes();
    let slot_bytes = clock.slot.to_le_bytes();
    
    randomness[16..24].copy_from_slice(&timestamp_bytes);
    randomness[24..32].copy_from_slice(&slot_bytes);
    
    // Hash everything together for final randomness
    let hash = solana_program::keccak::hash(&randomness);
    Ok(hash.to_bytes())
}

/// Extract winning number from randomness based on roulette type
fn extract_winning_number(randomness: &[u8; 32], roulette_type: crate::state::roulette::RouletteType) -> u8 {
    // Use first 4 bytes for the random value
    let random_u32 = u32::from_le_bytes([randomness[0], randomness[1], randomness[2], randomness[3]]);
    
    match roulette_type {
        crate::state::roulette::RouletteType::European => {
            (random_u32 % 37) as u8 // 0-36
        }
        crate::state::roulette::RouletteType::American => {
            (random_u32 % 38) as u8 // 0-37 (includes 00 as 37)
        }
    }
}

/// Fallback randomness generation for development/testing
/// This should NOT be used in production
pub fn generate_fallback_randomness(
    roulette: &RouletteAccount,
    clock: &Clock,
    caller: &Pubkey,
) -> Result<[u8; 32]> {
    msg!("WARNING: Using fallback randomness - NOT for production use");
    
    let mut entropy_sources = Vec::new();
    
    // Multiple entropy sources for secure fallback
    entropy_sources.extend_from_slice(&roulette.key().to_bytes());
    entropy_sources.extend_from_slice(&roulette.total_bets.to_le_bytes());
    entropy_sources.extend_from_slice(&roulette.total_bet_amount.to_le_bytes());
    entropy_sources.extend_from_slice(&roulette.created_at.to_le_bytes());
    entropy_sources.extend_from_slice(&clock.unix_timestamp.to_le_bytes());
    entropy_sources.extend_from_slice(&clock.slot.to_le_bytes());
    entropy_sources.extend_from_slice(&caller.to_bytes());
    
    // Hash all entropy sources
    let hash = solana_program::keccak::hash(&entropy_sources);
    Ok(hash.to_bytes())
}
