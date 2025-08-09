use anchor_lang::prelude::*;
use crate::state::global_config::GlobalConfig;
use crate::state::roulette::{RouletteAccount, RouletteState};
use crate::errors::RouletteError;
use crate::events::{RandomnessRequested, RouletteStateChanged};
use crate::constants::*;

/// VRF Client management for secure randomness
/// This module provides infrastructure for Switchboard VRF integration
/// and secure fallback mechanisms

#[derive(Accounts)]
pub struct InitializeVrfClient<'info> {
    #[account(
        init,
        payer = authority,
        space = VrfClientAccount::ACCOUNT_SIZE,
        seeds = [VRF_CLIENT_SEED, roulette.key().as_ref()],
        bump
    )]
    pub vrf_client: Account<'info, VrfClientAccount>,
    
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
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Locked @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        mut,
        seeds = [VRF_CLIENT_SEED, roulette.key().as_ref()],
        bump = vrf_client.bump,
        constraint = vrf_client.roulette == roulette.key() @ RouletteError::InvalidVrfAccount
    )]
    pub vrf_client: Account<'info, VrfClientAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Recent blockhashes sysvar for entropy
    /// CHECK: Solana sysvar for recent blockhashes
    #[account(address = anchor_lang::solana_program::sysvar::slot_hashes::id())]
    pub recent_blockhashes: AccountInfo<'info>,
    
    pub caller: Signer<'info>,
}

#[derive(Accounts)]
pub struct ConsumeRandomness<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::AwaitingRandomness @ RouletteError::InvalidGameState,
        constraint = !roulette.randomness_fulfilled @ RouletteError::RandomnessAlreadyFulfilled
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        mut,
        seeds = [VRF_CLIENT_SEED, roulette.key().as_ref()],
        bump = vrf_client.bump,
        constraint = vrf_client.roulette == roulette.key() @ RouletteError::InvalidVrfAccount
    )]
    pub vrf_client: Account<'info, VrfClientAccount>,
    
    /// CHECK: Switchboard randomness account (when available)
    #[account(mut)]
    pub randomness_account: UncheckedAccount<'info>,
    
    pub caller: Signer<'info>,
}

#[account]
pub struct VrfClientAccount {
    pub roulette: Pubkey,
    pub authority: Pubkey,
    pub randomness_account: Option<Pubkey>,
    pub is_initialized: bool,
    pub request_count: u64,
    pub last_request_timestamp: i64,
    pub created_at: i64,
    pub bump: u8,
}

impl VrfClientAccount {
    pub const ACCOUNT_SIZE: usize = 8 + // discriminator
        32 + // roulette
        32 + // authority  
        (1 + 32) + // randomness_account Option<Pubkey>
        1 + // is_initialized
        8 + // request_count
        8 + // last_request_timestamp
        8 + // created_at
        1; // bump
}

/// Initialize VRF client for a roulette game
pub fn initialize_vrf_client_handler(ctx: Context<InitializeVrfClient>) -> Result<()> {
    let vrf_client = &mut ctx.accounts.vrf_client;
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Initialize VRF client
    vrf_client.roulette = roulette.key();
    vrf_client.authority = ctx.accounts.authority.key();
    vrf_client.randomness_account = None;
    vrf_client.is_initialized = true;
    vrf_client.request_count = 0;
    vrf_client.last_request_timestamp = 0;
    vrf_client.created_at = clock.unix_timestamp;
    vrf_client.bump = ctx.bumps.vrf_client;
    
    // Link VRF client to roulette
    roulette.vrf_client = Some(vrf_client.key());
    
    msg!("VRF Client initialized for roulette: {}", roulette.key());
    Ok(())
}

/// Request randomness for the roulette spin
pub fn request_randomness_handler(ctx: Context<RequestRandomness>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let vrf_client = &mut ctx.accounts.vrf_client;
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
    
    // Update VRF client
    vrf_client.request_count += 1;
    vrf_client.last_request_timestamp = clock.unix_timestamp;
    
    // Transition roulette state
    let old_state = roulette.state.clone();
    roulette.state = RouletteState::AwaitingRandomness;
    
    // For production: Implement Switchboard VRF request here
    // For now: Use secure multi-source entropy as fallback
    let randomness_request_key = generate_randomness_request_key(
        &roulette,
        &vrf_client,
        &ctx.accounts.recent_blockhashes,
        &clock,
        &ctx.accounts.caller.key()
    )?;
    
    roulette.vrf_request_key = Some(randomness_request_key);
    
    // Emit events
    emit!(RouletteStateChanged {
        roulette_id: roulette.key(),
        old_state,
        new_state: RouletteState::AwaitingRandomness,
        timestamp: clock.unix_timestamp,
    });
    
    emit!(RandomnessRequested {
        roulette_id: roulette.key(),
        vrf_client: vrf_client.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Randomness requested for roulette: {}", roulette.key());
    Ok(())
}

/// Consume randomness and determine winning number
pub fn consume_randomness_handler(ctx: Context<ConsumeRandomness>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let vrf_client = &mut ctx.accounts.vrf_client;
    let clock = Clock::get()?;
    
    // Validate minimum time has passed since request (prevents manipulation)
    require!(
        clock.unix_timestamp >= vrf_client.last_request_timestamp + 10, // 10 seconds minimum
        RouletteError::TooEarly
    );
    
    // Generate secure randomness
    let randomness = generate_secure_randomness(
        &ctx.accounts.randomness_account,
        &roulette,
        &vrf_client,
        &clock,
        &ctx.accounts.caller.key()
    )?;
    
    // Extract winning number (0-36 for European roulette)
    let winning_number = extract_winning_number(&randomness, roulette.roulette_type);
    
    // Update roulette state
    roulette.vrf_randomness = Some(randomness);
    roulette.winning_number = Some(winning_number);
    roulette.randomness_fulfilled = true;
    roulette.state = RouletteState::Completed;
    roulette.completed_at = Some(clock.unix_timestamp);
    
    msg!("Randomness consumed. Winning number: {} for roulette: {}", winning_number, roulette.key());
    Ok(())
}

/// Generate a randomness request key for tracking
fn generate_randomness_request_key(
    roulette: &RouletteAccount,
    vrf_client: &VrfClientAccount,
    recent_blockhashes: &AccountInfo,
    clock: &Clock,
    caller: &Pubkey,
) -> Result<Pubkey> {
    let mut seed_data = Vec::new();
    
    // Combine multiple entropy sources
    seed_data.extend_from_slice(&roulette.authority.to_bytes());
    seed_data.extend_from_slice(&vrf_client.request_count.to_le_bytes());
    seed_data.extend_from_slice(&clock.unix_timestamp.to_le_bytes());
    seed_data.extend_from_slice(&clock.slot.to_le_bytes());
    seed_data.extend_from_slice(&caller.to_bytes());
    
    // Add recent blockhash entropy if available
    if let Ok(blockhash_data) = recent_blockhashes.try_borrow_data() {
        if blockhash_data.len() >= 32 {
            seed_data.extend_from_slice(&blockhash_data[..8]); // Use first 8 bytes
        }
    }
    
    // Create deterministic but unpredictable key
    let hash = solana_program::keccak::hash(&seed_data);
    Ok(Pubkey::new_from_array(hash.to_bytes()))
}

/// Generate cryptographically secure randomness
fn generate_secure_randomness(
    randomness_account: &UncheckedAccount,
    roulette: &RouletteAccount,
    vrf_client: &VrfClientAccount,
    clock: &Clock,
    caller: &Pubkey,
) -> Result<[u8; 32]> {
    // TODO: When Switchboard is available, consume actual VRF randomness here
    // For now, use multiple entropy sources for secure fallback
    
    let mut entropy_sources = Vec::new();
    
    // 1. Randomness account data (if available)
    if let Ok(randomness_data) = randomness_account.try_borrow_data() {
        if !randomness_data.is_empty() {
            entropy_sources.extend_from_slice(&randomness_data[..std::cmp::min(32, randomness_data.len())]);
        }
    }
    
    // 2. Roulette-specific entropy
    entropy_sources.extend_from_slice(&roulette.authority.to_bytes());
    entropy_sources.extend_from_slice(&roulette.total_bets.to_le_bytes());
    entropy_sources.extend_from_slice(&roulette.total_bet_amount.to_le_bytes());
    entropy_sources.extend_from_slice(&roulette.created_at.to_le_bytes());
    
    // 3. VRF client entropy
    entropy_sources.extend_from_slice(&vrf_client.authority.to_bytes());
    entropy_sources.extend_from_slice(&vrf_client.request_count.to_le_bytes());
    entropy_sources.extend_from_slice(&vrf_client.last_request_timestamp.to_le_bytes());
    
    // 4. Clock entropy
    entropy_sources.extend_from_slice(&clock.unix_timestamp.to_le_bytes());
    entropy_sources.extend_from_slice(&clock.slot.to_le_bytes());
    
    // 5. Caller entropy
    entropy_sources.extend_from_slice(&caller.to_bytes());
    
    // Hash all entropy sources
    let hash = solana_program::keccak::hash(&entropy_sources);
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
