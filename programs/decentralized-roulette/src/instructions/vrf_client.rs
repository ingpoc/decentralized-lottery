use anchor_lang::prelude::*;
// VRF functionality temporarily disabled - will be re-enabled with compatible switchboard version
// use switchboard_solana::VrfAccountData;
use crate::state::global_config::GlobalConfig;
use crate::state::roulette::{RouletteAccount, RouletteState};
use crate::errors::RouletteError;
use crate::events::{RandomnessRequested, RouletteStateChanged, VrfCompleted};
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
    
    /// Switchboard VRF account
    #[account(mut)]
    pub switchboard_vrf: AccountLoader<'info, VrfAccountData>,
    
    /// CHECK: Switchboard program
    #[account(address = "SW1TCH7qEPTdLsDHRgPuMQjbQxKdH2aBStViMFnt64f")]
    pub switchboard_program: AccountInfo<'info>,
    
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
    
    /// Switchboard VRF account
    #[account(mut)]
    pub switchboard_vrf: AccountLoader<'info, VrfAccountData>,
    
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
    #[account(address = "SW1TCH7qEPTdLsDHRgPuMQjbQxKdH2aBStViMFnt64f")]
    pub switchboard_program: AccountInfo<'info>,
    
    /// Token program for escrow payment
    pub token_program: Program<'info, anchor_spl::token::Token>,
    
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
    
    /// Switchboard VRF account containing the result
    #[account(mut)]
    pub switchboard_vrf: AccountLoader<'info, VrfAccountData>,
    
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

/// Generic handler alias for compatibility
pub use initialize_vrf_client_handler as handler;

/// Initialize VRF client for a roulette game
pub fn initialize_vrf_client_handler(ctx: Context<InitializeVrfClient>) -> Result<()> {
    let vrf_client = &mut ctx.accounts.vrf_client;
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Verify VRF account is owned by Switchboard
    require!(
        ctx.accounts.switchboard_vrf.to_account_info().owner == &"SW1TCH7qEPTdLsDHRgPuMQjbQxKdH2aBStViMFnt64f",
        RouletteError::InvalidVrfAccount
    );
    
    // Initialize VRF client
    vrf_client.roulette = roulette.key();
    vrf_client.authority = ctx.accounts.authority.key();
    vrf_client.randomness_account = Some(ctx.accounts.switchboard_vrf.key());
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
    
    // Create Switchboard VRF request
    let global_config = &ctx.accounts.global_config;
    let state_seeds: &[&[u8]] = &[
        GLOBAL_CONFIG_SEED,
        &[global_config.bump],
    ];
    
    // Create VRF request using Switchboard CPI
    let vrf_request_randomness = VrfRequestRandomness {
        authority: ctx.accounts.program_state.to_account_info(),
        vrf: ctx.accounts.switchboard_vrf.to_account_info(),
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
        vrf_client: vrf_client.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Switchboard VRF request submitted successfully");
    Ok(())
}

/// Consume randomness from Switchboard VRF result
pub fn consume_randomness_handler(ctx: Context<ConsumeRandomness>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Validate minimum time has passed since request (prevents manipulation)
    if let Some(request_time) = roulette.vrf_request_timestamp {
        require!(
            clock.unix_timestamp >= request_time + VRF_REQUEST_DELAY,
            RouletteError::TooEarly
        );
    }
    
    // Load VRF account data
    let vrf = ctx.accounts.switchboard_vrf.load()?;
    
    // Get VRF result
    let result_buffer = vrf.get_result()?;
    if result_buffer.is_empty() {
        return Err(RouletteError::RandomnessNotAvailable.into());
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
    
    // Calculate treasury fee
    let total_bet_amount = roulette.total_bet_amount;
    let treasury_fee = total_bet_amount
        .checked_mul(200u64)
        .and_then(|result| result.checked_div(10000))
        .ok_or(RouletteError::TreasuryFeeOverflow)?;
    roulette.treasury_fee_collected = treasury_fee;
    
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
        return Err(RouletteError::RandomnessNotAvailable.into());
    }
    
    let vrf_result = value[0];
    
    // Convert to bytes and create secure 256-bit randomness
    let mut randomness = [0u8; 32];
    
    // Use the VRF result as primary entropy
    let vrf_bytes = vrf_result.to_le_bytes();
    randomness[..16].copy_from_slice(&vrf_bytes);
    
    // Add secondary entropy from clock and other sources for full 256-bit
    let clock = Clock::get().map_err(|_| RouletteError::RandomnessGenerationFailed)?;
    let timestamp_bytes = clock.unix_timestamp.to_le_bytes();
    let slot_bytes = clock.slot.to_le_bytes();
    
    randomness[16..24].copy_from_slice(&timestamp_bytes);
    randomness[24..32].copy_from_slice(&slot_bytes);
    
    // Hash everything together for final randomness
    let hash = anchor_lang::solana_program::keccak::hash(&randomness);
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
    let hash = anchor_lang::solana_program::keccak::hash(&seed_data);
    Ok(Pubkey::new_from_array(hash.to_bytes()))
}

/// Generate cryptographically secure randomness - DEPRECATED: Use Switchboard VRF instead
fn generate_secure_randomness(
    randomness_account: &UncheckedAccount,
    roulette: &RouletteAccount,
    vrf_client: &VrfClientAccount,
    clock: &Clock,
    caller: &Pubkey,
) -> Result<[u8; 32]> {
    msg!("WARNING: Using fallback randomness generation - NOT for production use");
    
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
    let hash = anchor_lang::solana_program::keccak::hash(&entropy_sources);
    Ok(hash.to_bytes())
}

