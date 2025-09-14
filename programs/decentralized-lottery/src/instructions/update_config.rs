use anchor_lang::prelude::*;
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
}

pub fn update_config_handler(
    ctx: Context<UpdateConfig>, 
    new_fee_percentage: Option<u16>,
    new_admin: Option<Pubkey>,
    is_paused: Option<bool>
) -> Result<()> {
    let global_config = &mut ctx.accounts.global_config;
    let clock = Clock::get()?;
    
    let old_fee_percentage = Some(global_config.treasury_fee_percentage);
    let old_admin = Some(global_config.admin);
    let old_paused_state = Some(global_config.is_paused);
    
    let mut fee_percentage_updated = false;
    
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
        fee_percentage_updated || new_admin.is_some() || is_paused.is_some(),
        LotteryError::NoConfigChanges
    );
    
    // Emit configuration update event
    emit!(ConfigUpdated {
        authority: ctx.accounts.admin.key(),
        old_fee_percentage,
        new_fee_percentage,
        old_admin,
        new_admin,
        old_paused_state,
        new_paused_state: is_paused,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Configuration updated successfully");
    Ok(())
}
