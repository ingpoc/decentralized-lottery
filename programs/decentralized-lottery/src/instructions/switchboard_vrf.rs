use anchor_lang::prelude::*;
use switchboard_solana::*;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::{RandomnessRequested, RandomnessConsumed, LotteryStateChanged};

/// Switchboard VRF integration for lottery randomness
/// This provides production-ready Switchboard VRF integration

#[derive(Accounts)]
pub struct InitializeLotteryVrf<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.nonce.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Created @ LotteryError::InvalidLotteryState
    )]
    pub lottery_account: Account<'info, LotteryAccount>,
    
    #[account(
        seeds = [b"global_config_v2"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Switchboard VRF account
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Switchboard program
    #[account(address = "SW1TCH7qEPTdLsDHRgPuMQjbQxKdH2aBStViMFnt64f")]
    pub switchboard_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RequestLotteryRandomness<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.nonce.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Drawing @ LotteryError::InvalidLotteryState
    )]
    pub lottery_account: Account<'info, LotteryAccount>,
    
    #[account(
        seeds = [b"global_config_v2"],
        bump,
        constraint = global_config.admin == authority.key() @ LotteryError::AdminRequired
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
        seeds = [b"global_config_v2"],
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
    
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct ConsumeLotteryRandomness<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.nonce.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::AwaitingRandomness @ LotteryError::InvalidLotteryState,
        constraint = !lottery_account.randomness_fulfilled @ LotteryError::RandomnessAlreadyFulfilled
    )]
    pub lottery_account: Account<'info, LotteryAccount>,
    
    /// Switchboard VRF account containing the result
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
}

/// Initialize Switchboard VRF for a lottery
pub fn initialize_lottery_vrf_handler(ctx: Context<InitializeLotteryVrf>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let clock = Clock::get()?;
    
    // Verify VRF account is owned by Switchboard
    require!(
        ctx.accounts.vrf.to_account_info().owner == &"SW1TCH7qEPTdLsDHRgPuMQjbQxKdH2aBStViMFnt64f",
        LotteryError::InvalidVrfAccount
    );
    
    // Store VRF account reference
    lottery_account.vrf_client = Some(ctx.accounts.vrf.key());
    
    emit!(crate::events::VrfClientInitialized {
        lottery_id: lottery_account.key(),
        vrf_client: ctx.accounts.vrf.key(),
        vrf_account: ctx.accounts.vrf.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Switchboard VRF initialized for lottery: {}", lottery_account.key());
    Ok(())
}

/// Request randomness using Switchboard VRF
pub fn request_lottery_randomness_handler(ctx: Context<RequestLotteryRandomness>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let clock = Clock::get()?;
    
    // Transition lottery state
    let old_state = lottery_account.state.clone();
    lottery_account.state = LotteryState::AwaitingRandomness;
    
    // Prepare Switchboard VRF request
    let global_config = &ctx.accounts.global_config;
    let state_seeds: &[&[u8]] = &[
        b"global_config_v2",
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
    
    msg!("Requesting Switchboard VRF randomness for lottery: {}", lottery_account.key());
    
    // Invoke the Switchboard VRF request with PDA authority
    vrf_request_randomness.invoke_signed(
        ctx.accounts.switchboard_program.to_account_info(),
        &[state_seeds],
    )?;
    
    // Store request timestamp for validation
    lottery_account.vrf_request_key = Some(ctx.accounts.vrf.key());
    
    // Emit events
    emit!(LotteryStateChanged {
        lottery_id: lottery_account.key(),
        previous_state: old_state,
        new_state: LotteryState::AwaitingRandomness,
        timestamp: clock.unix_timestamp,
        total_tickets_sold: lottery_account.total_tickets,
        current_prize_pool: lottery_account.prize_pool,
    });
    
    emit!(RandomnessRequested {
        lottery_id: lottery_account.key(),
        vrf_client: ctx.accounts.vrf.key(),
        vrf_account: ctx.accounts.vrf.key(),
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Switchboard VRF request submitted successfully");
    Ok(())
}

/// Consume randomness from Switchboard VRF result
pub fn consume_lottery_randomness_handler(ctx: Context<ConsumeLotteryRandomness>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let clock = Clock::get()?;
    
    // Load VRF account data
    let vrf = ctx.accounts.vrf.load()?;
    
    // Get VRF result
    let result_buffer = vrf.get_result()?;
    if result_buffer.is_empty() {
        return Err(LotteryError::RandomnessNotAvailable.into());
    }
    
    // Extract randomness from Switchboard VRF result
    let randomness = extract_switchboard_randomness(&result_buffer)?;
    
    // Store the randomness and mark as fulfilled
    lottery_account.vrf_randomness = Some(randomness);
    lottery_account.randomness_fulfilled = true;
    
    // Emit events
    emit!(RandomnessConsumed {
        lottery_id: lottery_account.key(),
        vrf_client: ctx.accounts.vrf.key(),
        timestamp: clock.unix_timestamp,
        dice_result: 0, // Not applicable for lottery
    });
    
    msg!("Switchboard VRF consumed for lottery: {}", lottery_account.key());
    Ok(())
}

/// Extract randomness from Switchboard VRF result buffer
fn extract_switchboard_randomness(result_buffer: &[u8]) -> Result<[u8; 32]> {
    // Switchboard VRF returns 128-bit randomness
    // We need to extract and expand it to 256-bit for our use
    let value: &[u128] = bytemuck::cast_slice(result_buffer);
    
    if value.is_empty() {
        return Err(LotteryError::RandomnessNotAvailable.into());
    }
    
    let vrf_result = value[0];
    
    // Convert to bytes and create secure 256-bit randomness
    let mut randomness = [0u8; 32];
    
    // Use the VRF result as primary entropy
    let vrf_bytes = vrf_result.to_le_bytes();
    randomness[..16].copy_from_slice(&vrf_bytes);
    
    // Add secondary entropy from clock and other sources for full 256-bit
    let clock = Clock::get().map_err(|_| LotteryError::RandomnessGenerationFailed)?;
    let timestamp_bytes = clock.unix_timestamp.to_le_bytes();
    let slot_bytes = clock.slot.to_le_bytes();
    
    randomness[16..24].copy_from_slice(&timestamp_bytes);
    randomness[24..32].copy_from_slice(&slot_bytes);
    
    // Hash everything together for final randomness
    let hash = anchor_lang::solana_program::keccak::hash(&randomness);
    Ok(hash.to_bytes())
}
