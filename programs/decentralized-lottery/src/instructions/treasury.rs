use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

use crate::state::treasury::{GlobalConfig, Treasury};
use crate::errors::LotteryError;
use crate::events::TreasuryWithdrawal;

/// Account context for initializing the treasury.
/// This should be called once to set up the treasury configuration.
#[derive(Accounts)]
pub struct InitializeTreasury<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump,
        has_one = admin @ LotteryError::UnauthorizedAccess
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        init,
        payer = payer,
        space = Treasury::ACCOUNT_SIZE,
        seeds = [b"treasury_config"],
        bump
    )]
    pub treasury: Account<'info, Treasury>,
    
    pub admin: Signer<'info>,
    
    /// Multisig account that will control treasury withdrawals
    /// CHECK: Just storing the pubkey for validation in withdrawals
    pub multisig: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Account context for proposing a treasury withdrawal.
/// This creates a proposal that requires multisig approval.
#[derive(Accounts)]
pub struct ProposeWithdrawal<'info> {
    #[account(mut)]
    pub proposer: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        mut,
        seeds = [b"treasury_config"],
        bump,
        constraint = treasury.multisig == proposer.key() @ LotteryError::InvalidTreasuryMultisig,
    )]
    pub treasury: Account<'info, Treasury>,
    
    #[account(
        init,
        payer = proposer,
        space = WithdrawalProposal::ACCOUNT_SIZE,
        seeds = [b"withdrawal_proposal", treasury.key().as_ref(), &treasury.last_withdrawal_time.to_le_bytes()],
        bump
    )]
    pub withdrawal_proposal: Account<'info, WithdrawalProposal>,
    
    /// The token account where fees are held
    #[account(
        mut,
        address = global_config.treasury_token_account @ LotteryError::InvalidTokenAccount
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    
    /// The destination token account for the withdrawal
    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,
    
    pub system_program: Program<'info, System>,
}

/// Account context for approving a treasury withdrawal.
/// Requires multisig approval before funds can be withdrawn.
#[derive(Accounts)]
pub struct ApproveWithdrawal<'info> {
    #[account(mut)]
    pub approver: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        mut,
        seeds = [b"treasury_config"],
        bump,
        constraint = treasury.multisig == approver.key() || global_config.admin == approver.key() @ LotteryError::InvalidTreasuryMultisig,
    )]
    pub treasury: Account<'info, Treasury>,
    
    #[account(
        mut,
        seeds = [b"withdrawal_proposal", treasury.key().as_ref(), &treasury.last_withdrawal_time.to_le_bytes()],
        bump,
        constraint = withdrawal_proposal.is_active @ LotteryError::InvalidInstructionInput,
    )]
    pub withdrawal_proposal: Account<'info, WithdrawalProposal>,
    
    pub system_program: Program<'info, System>,
}

/// Account context for executing a treasury withdrawal.
/// Requires time-lock and approval requirements to be met.
#[derive(Accounts)]
pub struct ExecuteWithdrawal<'info> {
    #[account(mut)]
    pub executor: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        mut,
        seeds = [b"treasury_config"],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,
    
    #[account(
        mut,
        seeds = [b"withdrawal_proposal", treasury.key().as_ref(), &treasury.last_withdrawal_time.to_le_bytes()],
        bump,
        constraint = withdrawal_proposal.is_active @ LotteryError::InvalidInstructionInput,
        constraint = withdrawal_proposal.approvals >= withdrawal_proposal.required_approvals @ LotteryError::InvalidTreasuryMultisig,
        constraint = withdrawal_proposal.proposer == executor.key() || global_config.admin == executor.key() @ LotteryError::UnauthorizedAccess,
    )]
    pub withdrawal_proposal: Account<'info, WithdrawalProposal>,
    
    /// The token account where fees are held
    #[account(
        mut,
        address = global_config.treasury_token_account @ LotteryError::InvalidTokenAccount
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    
    /// The destination token account for the withdrawal
    #[account(
        mut,
        address = withdrawal_proposal.destination @ LotteryError::InvalidTokenAccount
    )]
    pub destination_token_account: Account<'info, TokenAccount>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Account context for emergency withdrawal by admin.
/// Bypasses multisig but still respects time-lock unless override flag is set.
#[derive(Accounts)]
pub struct EmergencyWithdrawal<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        seeds = [b"global_config"],
        bump,
        has_one = admin @ LotteryError::UnauthorizedAccess
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(
        mut,
        seeds = [b"treasury_config"],
        bump,
    )]
    pub treasury: Account<'info, Treasury>,
    
    /// The token account where fees are held
    #[account(
        mut,
        address = global_config.treasury_token_account @ LotteryError::InvalidTokenAccount
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    
    /// The destination token account for the withdrawal
    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,
    
    /// Token program
    pub token_program: Program<'info, Token>,
    
    /// System program
    pub system_program: Program<'info, System>,
}

/// Struct to represent a treasury withdrawal proposal
#[account]
pub struct WithdrawalProposal {
    pub proposer: Pubkey,
    pub amount: u64,
    pub destination: Pubkey,
    pub created_at: i64,
    pub required_approvals: u8,
    pub approvals: u8,
    pub approvers: Vec<Pubkey>,
    pub is_active: bool,
    pub is_executed: bool,
    pub bump: u8,
}

impl WithdrawalProposal {
    pub const MAX_APPROVERS: usize = 10;
    
    pub const ACCOUNT_SIZE: usize = 8 +  // Discriminator
        32 +  // proposer
        8 +   // amount
        32 +  // destination
        8 +   // created_at
        1 +   // required_approvals
        1 +   // approvals
        4 + (32 * Self::MAX_APPROVERS) +  // approvers vector
        1 +   // is_active
        1 +   // is_executed
        1;    // bump
        
    pub fn has_approver(&self, approver: &Pubkey) -> bool {
        self.approvers.iter().any(|a| a == approver)
    }
    
    pub fn add_approver(&mut self, approver: Pubkey) -> Result<()> {
        require!(!self.has_approver(&approver), LotteryError::InvalidInstructionInput);
        require!(self.approvers.len() < Self::MAX_APPROVERS, LotteryError::InvalidInstructionInput);
        
        self.approvers.push(approver);
        self.approvals += 1;
        
        Ok(())
    }
}

/// Initialize treasury with multisig and time-lock configuration
pub fn initialize_treasury_handler(ctx: Context<InitializeTreasury>, time_lock_seconds: i64) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    let clock = Clock::get()?;
    
    treasury.multisig = ctx.accounts.multisig.key();
    treasury.time_lock_seconds = time_lock_seconds;
    treasury.last_withdrawal_time = clock.unix_timestamp;
    treasury.treasury_balance = 0;
    
    msg!("Treasury initialized with multisig {} and time-lock {} seconds", 
        treasury.multisig, treasury.time_lock_seconds);
    
    Ok(())
}

/// Propose a withdrawal from the treasury
pub fn propose_withdrawal_handler(ctx: Context<ProposeWithdrawal>, amount: u64, required_approvals: u8) -> Result<()> {
    let treasury = &ctx.accounts.treasury;
    let withdrawal_proposal = &mut ctx.accounts.withdrawal_proposal;
    let treasury_token_account = &ctx.accounts.treasury_token_account;
    let clock = Clock::get()?;
    
    // Ensure the amount doesn't exceed the available balance
    require!(amount <= treasury_token_account.amount, LotteryError::InvalidInstructionInput);
    
    // Configure the withdrawal proposal
    withdrawal_proposal.proposer = ctx.accounts.proposer.key();
    withdrawal_proposal.amount = amount;
    withdrawal_proposal.destination = ctx.accounts.destination_token_account.key();
    withdrawal_proposal.created_at = clock.unix_timestamp;
    withdrawal_proposal.required_approvals = required_approvals;
    withdrawal_proposal.approvals = 1; // Proposer automatically counts as one approval
    withdrawal_proposal.approvers = vec![ctx.accounts.proposer.key()];
    withdrawal_proposal.is_active = true;
    withdrawal_proposal.is_executed = false;
    withdrawal_proposal.bump = ctx.bumps.withdrawal_proposal;
    
    msg!("Treasury withdrawal proposed: {} lamports to {}", 
        amount, withdrawal_proposal.destination);
    
    Ok(())
}

/// Approve a withdrawal proposal
pub fn approve_withdrawal_handler(ctx: Context<ApproveWithdrawal>) -> Result<()> {
    let withdrawal_proposal = &mut ctx.accounts.withdrawal_proposal;
    let approver = ctx.accounts.approver.key();
    
    // Add the approver if they haven't already approved
    withdrawal_proposal.add_approver(approver)?;
    
    msg!("Treasury withdrawal approved by {}. Current approvals: {}/{}", 
        approver, withdrawal_proposal.approvals, withdrawal_proposal.required_approvals);
    
    Ok(())
}

/// Execute a withdrawal after all approvals and time-lock
pub fn execute_withdrawal_handler(ctx: Context<ExecuteWithdrawal>) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    let withdrawal_proposal = &mut ctx.accounts.withdrawal_proposal;
    let treasury_token_account = &ctx.accounts.treasury_token_account;
    let destination_token_account = &ctx.accounts.destination_token_account;
    let clock = Clock::get()?;
    
    // Check time-lock constraint
    let time_since_last_withdrawal = clock.unix_timestamp - treasury.last_withdrawal_time;
    require!(
        time_since_last_withdrawal >= treasury.time_lock_seconds,
        LotteryError::TreasuryWithdrawalTimeLockNotReached
    );
    
    // Ensure the proposal is active and not already executed
    require!(withdrawal_proposal.is_active && !withdrawal_proposal.is_executed, LotteryError::InvalidInstructionInput);
    
    // Ensure the amount doesn't exceed the available balance
    require!(withdrawal_proposal.amount <= treasury_token_account.amount, LotteryError::InvalidInstructionInput);
    
    // Transfer the tokens
    let cpi_accounts = Transfer {
        from: treasury_token_account.to_account_info(),
        to: destination_token_account.to_account_info(),
        authority: ctx.accounts.global_config.to_account_info(),
    };
    
    let seeds = &[
        b"global_config".as_ref(),
        &[ctx.bumps.global_config],
    ];
    let signer = &[&seeds[..]];
    
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );
    
    token::transfer(cpi_ctx, withdrawal_proposal.amount)?;
    
    // Update the treasury information
    treasury.last_withdrawal_time = clock.unix_timestamp;
    treasury.treasury_balance = treasury.treasury_balance.saturating_sub(withdrawal_proposal.amount);
    
    // Mark the proposal as executed
    withdrawal_proposal.is_active = false;
    withdrawal_proposal.is_executed = true;
    
    // Emit withdrawal event
    emit!(TreasuryWithdrawal {
        treasury: treasury.key(),
        amount: withdrawal_proposal.amount,
        destination: destination_token_account.key(),
        timestamp: clock.unix_timestamp,
        is_emergency: false,
    });
    
    msg!("Treasury withdrawal executed: {} lamports to {}", 
        withdrawal_proposal.amount, destination_token_account.key());
    
    Ok(())
}

/// Emergency withdrawal by admin
pub fn emergency_withdrawal_handler(
    ctx: Context<EmergencyWithdrawal>, 
    amount: u64,
    override_timelock: bool
) -> Result<()> {
    let treasury = &mut ctx.accounts.treasury;
    let treasury_token_account = &ctx.accounts.treasury_token_account;
    let destination_token_account = &ctx.accounts.destination_token_account;
    let clock = Clock::get()?;
    
    // Check time-lock constraint unless override flag is set
    if !override_timelock {
        let time_since_last_withdrawal = clock.unix_timestamp - treasury.last_withdrawal_time;
        require!(
            time_since_last_withdrawal >= treasury.time_lock_seconds,
            LotteryError::TreasuryWithdrawalTimeLockNotReached
        );
    }
    
    // Ensure the amount doesn't exceed the available balance
    require!(amount <= treasury_token_account.amount, LotteryError::InvalidInstructionInput);
    
    // Transfer the tokens
    let cpi_accounts = Transfer {
        from: treasury_token_account.to_account_info(),
        to: destination_token_account.to_account_info(),
        authority: ctx.accounts.global_config.to_account_info(),
    };
    
    let seeds = &[
        b"global_config".as_ref(),
        &[ctx.bumps.global_config],
    ];
    let signer = &[&seeds[..]];
    
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer,
    );
    
    token::transfer(cpi_ctx, amount)?;
    
    // Update the treasury information
    treasury.last_withdrawal_time = clock.unix_timestamp;
    treasury.treasury_balance = treasury.treasury_balance.saturating_sub(amount);
    
    // Emit emergency withdrawal event
    emit!(TreasuryWithdrawal {
        treasury: treasury.key(),
        amount,
        destination: destination_token_account.key(),
        timestamp: clock.unix_timestamp,
        is_emergency: true,
    });
    
    msg!("Emergency treasury withdrawal executed: {} lamports to {}", 
        amount, destination_token_account.key());
    
    Ok(())
}

