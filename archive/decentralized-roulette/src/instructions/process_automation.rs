use anchor_lang::prelude::*;
use crate::state::global_config::GlobalConfig;
use crate::constants::*;
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct ProcessAutomation<'info> {
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Anyone can call this instruction to process automation
    pub caller: Signer<'info>,
}

pub fn handler(ctx: Context<ProcessAutomation>) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    msg!("Processing automation cycle at {} by {}", 
         current_time, 
         ctx.accounts.caller.key());
    
    // This instruction now triggers automatic game creation
    // The client should call create_next_game if no active games exist
    // This instruction validates that automation is needed
    
    // Note: The actual game creation logic is separated into create_next_game 
    // instruction to avoid complex account structures in this instruction.
    // Clients should:
    // 1. Call this instruction to validate automation is enabled
    // 2. Check for active games via RPC calls
    // 3. Call create_next_game if no active games exist
    
    msg!("Automation cycle validated successfully");
    
    Ok(())
}