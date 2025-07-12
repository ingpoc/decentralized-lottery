use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::state::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::ConfigUpdated;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"global_config_v2"],
        bump,
        constraint = global_config.admin == admin.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub admin: Signer<'info>,
    
    /// New USDC mint (optional update)
    #[account(
        constraint = new_usdc_mint.mint.is_initialized @ LotteryError::InvalidTokenAccount
    )]
    pub new_usdc_mint: Option<Account<'info, TokenAccount>>,
    
    /// New treasury token account (optional update)  
    #[account(
        constraint = new_treasury_token_account.owner == global_config.admin @ LotteryError::InvalidTokenAccount
    )]
    pub new_treasury_token_account: Option<Account<'info, TokenAccount>>,
    
    pub token_program: Program<'info, Token>,
}

#[event]
pub struct ConfigUpdated {
    pub admin: Pubkey,
    pub usdc_mint_updated: bool,
    pub treasury_updated: bool,
    pub fee_percentage_updated: bool,
    pub new_fee_percentage: Option<u16>,
    pub timestamp: i64,
}

pub fn handler(
    ctx: Context<UpdateConfig>, 
    new_fee_percentage: Option<u16>,
    new_admin: Option<Pubkey>,
    is_paused: Option<bool>
) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let clock = Clock::get()?;
    
    let mut usdc_mint_updated = false;
    let mut treasury_updated = false;
    let mut fee_percentage_updated = false;
    
    // Update USDC mint if provided
    if let Some(new_mint) = &ctx.accounts.new_usdc_mint {
        require!(
            new_mint.mint != global_config.usdc_mint,
            LotteryError::InvalidTokenAccount
        );
        global_config.usdc_mint = new_mint.mint;
        usdc_mint_updated = true;
        msg!("USDC mint updated to: {}", new_mint.mint);
    }
    
    // Update treasury token account if provided
    if let Some(new_treasury) = &ctx.accounts.new_treasury_token_account {
        require!(
            new_treasury.key() != global_config.treasury_token_account,
            LotteryError::InvalidTokenAccount
        );
        require!(
            new_treasury.mint == global_config.usdc_mint,
            LotteryError::InvalidTokenAccount
        );
        global_config.treasury_token_account = new_treasury.key();
        treasury_updated = true;
        msg!("Treasury token account updated to: {}", new_treasury.key());
    }
    
    // Update treasury fee percentage if provided
    if let Some(fee) = new_fee_percentage {
        require!(
            fee <= 1000, // Maximum 10% (in basis points)
            LotteryError::InvalidFeePercentage
        );
        require!(
            fee != global_config.treasury_fee_percentage,
            LotteryError::InvalidFeePercentage
        );
        global_config.treasury_fee_percentage = fee;
        fee_percentage_updated = true;
        msg!("Treasury fee percentage updated to: {}%", fee as f64 / 100.0);
    }
    
    // Update admin if provided (be very careful with this!)
    if let Some(new_admin_key) = new_admin {
        require!(
            new_admin_key != global_config.admin,
            LotteryError::InvalidAuthority
        );
        global_config.admin = new_admin_key;
        msg!("Admin updated to: {}", new_admin_key);
    }
    
    // Update pause state if provided
    if let Some(pause_state) = is_paused {
        global_config.is_paused = pause_state;
        msg!("System pause state updated to: {}", pause_state);
    }
    
    // Require at least one update
    require!(
        usdc_mint_updated || treasury_updated || fee_percentage_updated || 
        new_admin.is_some() || is_paused.is_some(),
        LotteryError::NoConfigChanges
    );
    
    // Emit configuration update event
    emit!(ConfigUpdated {
        admin: ctx.accounts.admin.key(),
        usdc_mint_updated,
        treasury_updated,
        fee_percentage_updated,
        new_fee_percentage,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Configuration updated successfully");
    Ok(())
}
