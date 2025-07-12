use anchor_lang::prelude::*;
use crate::state::global_config::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::{EmergencyPause, LotteryCancelled};
use crate::state::lottery::{LotteryAccount, LotteryState};

const GLOBAL_CONFIG_SEED: &[u8] = b"global_config";

#[derive(Accounts)]
pub struct EmergencyPauseToggle<'info> {
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = global_config.admin == authority.key() @ LotteryError::InvalidAuthority
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Only the global admin can pause/unpause
    pub authority: Signer<'info>,
}

/// Emergency pause/unpause functionality
/// Only callable by the global admin
pub fn emergency_pause_toggle_handler(ctx: Context<EmergencyPauseToggle>, pause: bool) -> Result<()> {
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
pub struct ForceCancelLottery<'info> {
    #[account(
        mut,
        constraint = lottery.state != LotteryState::Completed @ LotteryError::LotteryCompleted,
        constraint = lottery.state != LotteryState::Cancelled @ LotteryError::LotteryCancelled
    )]
    pub lottery: Account<'info, LotteryAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = global_config.admin == authority.key() @ LotteryError::InvalidAuthority
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Only the global admin can force cancel lotteries
    pub authority: Signer<'info>,
}

/// Force cancel a lottery in emergency situations
/// Only callable by the global admin
pub fn force_cancel_lottery_handler(ctx: Context<ForceCancelLottery>, reason: String) -> Result<()> {
    let lottery = &mut ctx.accounts.lottery;
    let clock = Clock::get()?;
    
    let old_state = lottery.state.clone();
    lottery.state = LotteryState::Cancelled;
    lottery.completed_at = Some(clock.unix_timestamp);
    
    // Calculate total refunds (all tickets sold * ticket price)
    let total_refunds = lottery.total_tickets.checked_mul(lottery.ticket_price)
        .ok_or(LotteryError::ArithmeticOverflow)?;
    
    // Emit events
    emit!(crate::events::LotteryStateChanged {
        lottery_id: lottery.key(),
        previous_state: old_state,
        new_state: LotteryState::Cancelled,
        timestamp: clock.unix_timestamp,
        total_tickets_sold: lottery.total_tickets,
        current_prize_pool: lottery.prize_pool,
    });
    
    emit!(LotteryCancelled {
        lottery_id: lottery.key(),
        reason,
        total_refunds,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("🚨 LOTTERY FORCE CANCELLED by admin: {}", lottery.key());
    
    Ok(())
}
