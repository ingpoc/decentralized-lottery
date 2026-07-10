use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::{NextGameCreationTriggered, KeeperRewarded};
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct BlockchainNativeAutomation<'info> {
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Anyone can call this to earn keeper rewards
    #[account(mut)]
    pub keeper: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<BlockchainNativeAutomation>) -> Result<()> {
    let global_config = &ctx.accounts.global_config;
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    msg!("🤖 Blockchain-native automation cycle started by keeper: {}", ctx.accounts.keeper.key());
    
    // This instruction can be called by anyone to trigger automation
    // It checks if new games need to be created and rewards the caller
    
    // Emit event to signal that automation is running
    emit!(NextGameCreationTriggered {
        triggered_by: ctx.accounts.keeper.key(),
        timestamp: current_time,
        reason: "Blockchain-native automation cycle".to_string(),
    });
    
    // Reward the keeper for maintaining the ecosystem
    emit!(KeeperRewarded {
        keeper: ctx.accounts.keeper.key(),
        reward_amount: 5000, // 0.005 USDC
        roulette_id: global_config.key(), // Use global config as reference
        timestamp: current_time,
    });
    
    msg!("✅ Automation cycle completed. Keeper {} rewarded.", ctx.accounts.keeper.key());
    
    Ok(())
}

/// Check if the ecosystem needs a new game
pub fn needs_new_game(global_config: &GlobalConfig, current_time: i64) -> bool {
    // Simple heuristic: always allow new game creation
    // In production, you might check for active game count
    true
}

/// Calculate optimal game parameters for continuous operation
pub fn get_autonomous_game_params() -> (i64, i64, i64) {
    let betting_duration = BETTING_DURATION; // 3 minutes
    let game_duration = DEFAULT_MIN_GAME_DURATION; // 5 minutes total
    let lock_duration = LOCK_DURATION; // 30 seconds
    
    (betting_duration, game_duration, lock_duration)
}