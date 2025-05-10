use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use switchboard_v2::{
    VrfAccountData, 
    OracleQueueAccountData, 
    PermissionAccountData,
    SbState,
};
use switchboard_solana::VrfRequestRandomness;

use crate::state::vrf::VrfClientState;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::errors::LotteryError;
use crate::events::{VrfClientInitialized, RandomnessRequested, RandomnessConsumed};

/// Instruction context for initializing a VRF client
/// This should be called when creating a lottery or preparing for a draw
#[derive(Accounts)]
pub struct InitVrfClient<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The authority of the lottery (admin)
    #[account(
        constraint = payer.key() == lottery_account.authority @ LotteryError::UnauthorizedAccess
    )]
    pub authority: Signer<'info>,

    /// Lottery account associated with this VRF request
    #[account(
        mut,
        constraint = 
            lottery_account.state == LotteryState::Open || 
            lottery_account.state == LotteryState::Locked
            @ LotteryError::InvalidLotteryState
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// The VRF client state account to be initialized
    #[account(
        init,
        seeds = [
            b"vrf-client", 
            lottery_account.key().as_ref()
        ],
        bump,
        payer = payer,
        space = VrfClientState::ACCOUNT_SIZE
    )]
    pub vrf_client: Account<'info, VrfClientState>,

    /// The Switchboard VRF account to use for randomness requests
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,

    /// The oracle queue associated with the VRF account
    pub oracle_queue: AccountLoader<'info, OracleQueueAccountData>,

    /// Queue authority
    pub queue_authority: UncheckedAccount<'info>,

    /// The permission account (granted to the VRF)
    pub permission: AccountLoader<'info, PermissionAccountData>,

    /// Escrow account for the VRF (if needed)
    #[account(mut)]
    pub escrow: Account<'info, TokenAccount>,

    /// Program state account for Switchboard
    pub program_state: AccountLoader<'info, SbState>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Switchboard program ID
    /// CHECK: The address should match the Switchboard program ID
    #[account(address = switchboard_v2::ID)]
    pub switchboard_program: AccountInfo<'info>,
}

/// Context for requesting randomness from Switchboard VRF
#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The authority of the lottery (admin or delegated keeper)
    #[account(
        constraint = payer.key() == lottery_account.authority @ LotteryError::UnauthorizedAccess
    )]
    pub authority: Signer<'info>,

    /// The lottery account that is requesting randomness
    #[account(
        mut,
        constraint = lottery_account.state == LotteryState::Locked @ LotteryError::InvalidStateTransition,
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// The VRF client state account
    #[account(
        mut, 
        seeds = [
            b"vrf-client", 
            lottery_account.key().as_ref()
        ],
        bump = vrf_client.bump,
        constraint = vrf_client.lottery_account == lottery_account.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub vrf_client: Account<'info, VrfClientState>,

    /// The Switchboard VRF account to use for randomness
    #[account(
        mut,
        constraint = vrf_client.vrf_account == vrf.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub vrf: AccountLoader<'info, VrfAccountData>,

    /// Oracle queue account
    pub oracle_queue: AccountLoader<'info, OracleQueueAccountData>,

    /// Queue authority
    pub queue_authority: UncheckedAccount<'info>,

    /// Data buffer account
    #[account(mut)]
    /// CHECK: Validated in the VRF CPI
    pub data_buffer: AccountInfo<'info>,

    /// Permission account
    pub permission: AccountLoader<'info, PermissionAccountData>,

    /// Escrow account for payment
    #[account(mut)]
    pub escrow: Account<'info, TokenAccount>,

    /// Payer's token account for funding the request
    #[account(mut)]
    pub payer_token_wallet: Account<'info, TokenAccount>,

    /// Program state account
    pub program_state: AccountLoader<'info, SbState>,

    /// Recent blockhashes sysvar
    /// CHECK: Validated in the VRF CPI
    pub recent_blockhashes: UncheckedAccount<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Switchboard program
    /// CHECK: Validated through the address constraint
    #[account(address = switchboard_v2::ID)]
    pub switchboard_program: AccountInfo<'info>,
}

/// Context for consuming the randomness result from Switchboard
#[derive(Accounts)]
pub struct ConsumeRandomness<'info> {
    /// Payer of the transaction (usually the lottery authority)
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The authority of the lottery (admin or delegated keeper)
    #[account(
        constraint = authority.key() == lottery_account.authority @ LotteryError::UnauthorizedAccess
    )]
    pub authority: Signer<'info>,

    /// The lottery account that requested the randomness
    #[account(
        mut,
        constraint = lottery_account.state == LotteryState::AwaitingRandomness @ LotteryError::InvalidStateTransition,
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// The VRF client state account
    #[account(
        mut,
        seeds = [
            b"vrf-client", 
            lottery_account.key().as_ref()
        ],
        bump = vrf_client.bump,
        constraint = vrf_client.lottery_account == lottery_account.key() @ LotteryError::InvalidAccountOwner,
        constraint = vrf_client.vrf_account == vrf.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub vrf_client: Account<'info, VrfClientState>,

    /// The Switchboard VRF account containing the randomness result
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
}

/// Handler to initialize a VRF client for a lottery
pub fn init_vrf_client_handler(ctx: Context<InitVrfClient>) -> Result<()> {
    let vrf_client = &mut ctx.accounts.vrf_client;
    let lottery_account = &ctx.accounts.lottery_account;
    let clock = Clock::get()?;

    // Initialize VRF client state
    vrf_client.bump = *ctx.bumps.get("vrf_client").ok_or(LotteryError::PDADerivationError)?;
    vrf_client.result_buffer = [0u8; 32];
    vrf_client.dice_result = 0;
    vrf_client.timestamp = clock.unix_timestamp;
    vrf_client.vrf_account = ctx.accounts.vrf.key();
    vrf_client.lottery_account = lottery_account.key();
    vrf_client.escrow_account = Some(ctx.accounts.escrow.key());
    vrf_client.is_consumed = false;

    msg!("Initialized VRF client for lottery {}", lottery_account.key());
    
    // Emit event
    emit!(VrfClientInitialized {
        lottery_id: lottery_account.key(),
        vrf_client: vrf_client.key(),
        vrf_account: vrf_client.vrf_account,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Handler to request randomness from Switchboard VRF
pub fn request_randomness_handler(ctx: Context<RequestRandomness>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let vrf_client = &mut ctx.accounts.vrf_client;
    let clock = Clock::get()?;

    // Mark lottery as awaiting randomness
    lottery_account.state = LotteryState::AwaitingRandomness;
    
    // Update the VRF client timestamp
    vrf_client.timestamp = clock.unix_timestamp;
    
    // Make the randomness request via CPI to Switchboard
    let switchboard_request = VrfRequestRandomness {
        authority: ctx.accounts.authority.to_account_info(),
        vrf: ctx.accounts.vrf.to_account_info(),
        oracle_queue: ctx.accounts.oracle_queue.to_account_info(),
        queue_authority: ctx.accounts.queue_authority.to_account_info(),
        data_buffer: ctx.accounts.data_buffer.to_account_info(),
        permission: ctx.accounts.permission.to_account_info(),
        escrow: ctx.accounts.escrow.clone(),
        payer_wallet: ctx.accounts.payer_token_wallet.clone(),
        payer_authority: ctx.accounts.payer.to_account_info(),
        recent_blockhashes: ctx.accounts.recent_blockhashes.to_account_info(),
        program_state: ctx.accounts.program_state.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
    };

    // Execute the request
    switchboard_request.invoke(
        ctx.accounts.switchboard_program.clone(),
        ctx.accounts.vrf_client.key(),
        None, // Optional custom Callback
    ).map_err(|_| LotteryError::RandomnessGenerationFailed)?;

    msg!("Requested randomness for lottery {}", lottery_account.key());
    
    // Emit event
    emit!(RandomnessRequested {
        lottery_id: lottery_account.key(),
        vrf_client: vrf_client.key(),
        vrf_account: vrf_client.vrf_account,
        timestamp: clock.unix_timestamp,
    });
    
    Ok(())
}

/// Handler to consume randomness from Switchboard VRF result
pub fn consume_randomness_handler(ctx: Context<ConsumeRandomness>) -> Result<()> {
    let vrf_client = &mut ctx.accounts.vrf_client;
    let lottery_account = &ctx.accounts.lottery_account;
    let vrf = ctx.accounts.vrf.load()?;
    let clock = Clock::get()?;

    // Check if VRF callback has been triggered
    if !vrf.has_callback_triggered() {
        return Err(LotteryError::RandomnessGenerationFailed.into());
    }

    // Check if randomness has already been consumed
    if vrf_client.is_consumed {
        return Err(LotteryError::RandomnessGenerationFailed.into());
    }

    // Extract the randomness result
    let result_buffer = vrf.get_result()
        .ok_or(LotteryError::RandomnessGenerationFailed)?;
    
    // Store the randomness in the client state
    vrf_client.result_buffer.copy_from_slice(&result_buffer[0..32]);
    
    // Calculate dice result (u64 from first 8 bytes)
    let mut result_bytes = [0u8; 8];
    result_bytes.copy_from_slice(&result_buffer[0..8]);
    let dice_result = u64::from_le_bytes(result_bytes);
    vrf_client.dice_result = dice_result;
    
    // Update timestamp
    vrf_client.timestamp = clock.unix_timestamp;
    
    // Mark randomness as consumed
    vrf_client.is_consumed = true;

    msg!("Randomness consumed: dice result = {}", dice_result);
    
    // Emit event
    emit!(RandomnessConsumed {
        lottery_id: lottery_account.key(),
        vrf_client: vrf_client.key(),
        timestamp: clock.unix_timestamp,
        dice_result: dice_result,
    });
    
    Ok(())
}

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use switchboard_v2::{
    VrfAccountData, 
    OracleQueueAccountData, 
    PermissionAccountData,
    SbState,
};
use switchboard_solana::VrfRequestRandomness;

use crate::state::vrf::VrfClientState;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::errors::LotteryError;

/// Instruction to initialize a new VRF client for a lottery.
/// This should be called when creating a lottery or preparing for a draw.
#[derive(Accounts)]
pub struct InitVrfClient<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// Lottery account associated with this VRF request
    #[account(
        mut,
        constraint = 
            lottery_account.state == LotteryState::Open || 
            lottery_account.state == LotteryState::Locked
            @ LotteryError::InvalidLotteryState
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// The VRF client state account to be initialized
    #[account(
        init,
        seeds = [
            b"vrf-client", 
            lottery_account.key().as_ref()
        ],
        bump,
        payer = payer,
        space = VrfClientState::ACCOUNT_SIZE
    )]
    pub vrf_client: Account<'info, VrfClientState>,

    /// The Switchboard VRF account to use for randomness requests
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,

    /// The oracle queue associated with the VRF account
    pub oracle_queue: AccountLoader<'info, OracleQueueAccountData>,

    /// Queue authority (optional)
    pub queue_authority: UncheckedAccount<'info>,

    /// The permission account (granted to the VRF)
    pub permission: AccountLoader<'info, PermissionAccountData>,

    /// Escrow account for the VRF (if needed)
    #[account(mut)]
    pub escrow: Account<'info, TokenAccount>,

    /// Program state account for Switchboard
    pub program_state: AccountLoader<'info, SbState>,

    /// System program
    pub system_program: Program<'info, System>,

    /// Switchboard program ID
    /// CHECK: The address should match the Switchboard program ID
    #[account(address = switchboard_v2::ID)]
    pub switchboard_program: AccountInfo<'info>,
}

/// Context for requesting randomness from Switchboard VRF
#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The lottery account that is requesting randomness
    #[account(
        mut,
        constraint = lottery_account.state == LotteryState::Locked @ LotteryError::InvalidStateTransition,
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// The VRF client state account
    #[account(
        mut, 
        seeds = [
            b"vrf-client", 
            lottery_account.key().as_ref()
        ],
        bump = vrf_client.bump,
    )]
    pub vrf_client: Account<'info, VrfClientState>,

    /// The Switchboard VRF account to use for randomness
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,

    /// Oracle queue account
    pub oracle_queue: AccountLoader<'info, OracleQueueAccountData>,

    /// Queue authority
    pub queue_authority: UncheckedAccount<'info>,

    /// Data buffer account
    #[account(mut)]
    pub data_buffer: AccountInfo<'info>,

    /// Permission account
    pub permission: AccountLoader<'info, PermissionAccountData>,

    /// Escrow account for payment
    #[account(mut)]
    pub escrow: Account<'info, TokenAccount>,

    /// Payer's token account for funding the request
    #[account(mut)]
    pub payer_token_wallet: Account<'info, TokenAccount>,

    /// Program state account
    pub program_state: AccountLoader<'info, SbState>,

    /// Recent blockhashes sysvar
    pub recent_blockhashes: UncheckedAccount<'info>,

    /// Token program
    pub token_program: Program<'info, Token>,

    /// Switchboard program
    /// CHECK: The address should match the Switchboard program ID
    #[account(address = switchboard_v2::ID)]
    pub switchboard_program: AccountInfo<'info>,
}

/// Context for consuming the randomness result from Switchboard
#[derive(Accounts)]
pub struct ConsumeRandomness<'info> {
    #[account(
        constraint = payer.key() == lottery_account.authority @ LotteryError::UnauthorizedAccess
    )]
    pub payer: Signer<'info>,

    /// The lottery account that requested the randomness
    #[account(
        mut,
        constraint = lottery_account.state == LotteryState::AwaitingRandomness @ LotteryError::InvalidStateTransition,
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// The VRF client state account
    #[account(
        mut,
        seeds = [
            b"vrf-client", 
            lottery_account.key().as_ref()
        ],
        bump = vrf_client.bump,
        constraint = vrf_client.lottery_account == lottery_account.key() @ LotteryError::InvalidAccountOwner,
        constraint = vrf_client.vrf_account == vrf.key() @ LotteryError::InvalidAccountOwner,
    )]
    pub vrf_client: Account<'info, VrfClientState>,

    /// The Switchboard VRF account containing the randomness result
    #[account(mut)]
    pub vrf: AccountLoader<'info, VrfAccountData>,
}

/// Handler to initialize a VRF client for a lottery
pub fn init_vrf_client_handler(ctx: Context<InitVrfClient>) -> Result<()> {
    let vrf_client = &mut ctx.accounts.vrf_client;
    let lottery_account = &ctx.accounts.lottery_account;
    let clock = Clock::get()?;

    // Initialize VRF client state
    vrf_client.bump = *ctx.bumps.get("vrf_client").ok_or(LotteryError::PDADerivationError)?;
    vrf_client.result_buffer = [0u8; 32];
    vrf_client.dice_result = 0;
    vrf_client.timestamp = clock.unix_timestamp;
    vrf_client.vrf_account = ctx.accounts.vrf.key();
    vrf_client.lottery_account = lottery_account.key();
    vrf_client.escrow_account = Some(ctx.accounts.escrow.key());
    vrf_client.is_consumed = false;

    msg!("Initialized VRF client for lottery {}", lottery_account.key());
    
    Ok(())
}

/// Handler to request randomness from Switchboard VRF
pub fn request_randomness_handler(ctx: Context<RequestRandomness>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let vrf_client = &mut ctx.accounts.vrf_client;
    let clock = Clock::get()?;

    // Mark lottery as awaiting randomness
    lottery_account.state = LotteryState::AwaitingRandomness;
    
    // Update the VRF client timestamp
    vrf_client.timestamp = clock.unix_timestamp;
    
    // Make the randomness request via CPI to Switchboard
    let switchboard_request = VrfRequestRandomness {
        authority: ctx.accounts.payer.to_account_info(),
        vrf: ctx.accounts.vrf.to_account_info(),
        oracle_queue: ctx.accounts.oracle_queue.to_account_info(),
        queue_authority: ctx.accounts.queue_authority.to_account_info(),
        data_buffer: ctx.accounts.data_buffer.to_account_info(),
        permission: ctx.accounts.permission.to_account_info(),
        escrow: ctx.accounts.escrow.clone(),
        payer_wallet: ctx.accounts.payer_token_wallet.clone(),
        payer_authority: ctx.accounts.payer.to_account_info(),
        recent_blockhashes: ctx.accounts.recent_blockhashes.to_account_info(),
        program_state: ctx.accounts.program_state.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
    };

    // Execute the request
    switchboard_request.invoke(
        ctx.accounts.switchboard_program.clone(),
        ctx.accounts.vrf_client.key(),
        None, // Optional custom Callback
    ).map_err(|_| LotteryError::RandomnessGenerationFailed)?;

    msg!("Requested randomness for lottery {}", lottery_account.key());
    
    Ok(())
}

/// Handler to consume randomness from Switchboard VRF result
pub fn consume_randomness_handler(ctx: Context<ConsumeRandomness>) -> Result<()> {
    let vrf_client = &mut ctx.accounts.vrf_client;
    let lottery_account = &mut ctx.accounts.lottery_account;
    let vrf = ctx.accounts.vrf.load()?;
    let clock = Clock::get()?;

    // Check if VRF callback has been triggered
    if !vrf.has_callback_triggered() {
        return Err(LotteryError::RandomnessGenerationFailed.into());
    }

    // Check if randomness has already been consumed
    if vrf_client.is_consumed {
        return Err(LotteryError::RandomnessGenerationFailed.into());
    }

    // Extract the randomness result
    let result_buffer = vrf.get_result()
        .ok_or(LotteryError::RandomnessGenerationFailed)?;
    
    // Store the randomness in the client state
    vrf_client.result_buffer.copy_from_slice(&result_buffer[0..32]);
    
    // Calculate dice result (u64 from first 8 bytes)
    let mut result_bytes = [0u8; 8];
    result_bytes.copy_from_slice(&result_buffer[0..8]);
    let dice_result = u64::from_le_bytes(result_bytes);
    vrf_client.dice_result = dice_result;
    
    // Update timestamp
    vrf_client.timestamp = clock.unix_timestamp;
    
    // Mark randomness as consumed
    vrf_client.is_consumed = true;

    msg!("Randomness consumed: dice result = {}", dice_result);
    
    // Return the randomness for further processing in the settle_randomness instruction
    // The caller should now call settle_randomness to complete the lottery draw
    
    Ok(())
}

