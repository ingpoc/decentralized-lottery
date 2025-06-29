use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::RouletteCancelled;
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct CancelRoulette<'info> {
    #[account(
        mut,
        constraint = roulette.state != RouletteState::Completed @ RouletteError::InvalidGameState,
        constraint = roulette.state != RouletteState::Cancelled @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = authority.key() == global_config.authority @ RouletteError::InvalidAuthority
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Only the global authority can cancel roulettes
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// CHECK: Roulette token account (holds bets to be refunded)
    #[account(mut)]
    pub roulette_token_account: AccountInfo<'info>,
    
    /// CHECK: Treasury token account (receives any remaining funds)
    #[account(mut)]
    pub treasury_token_account: AccountInfo<'info>,
    
    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelRouletteArgs {
    pub reason: String,
}

pub fn handler(ctx: Context<CancelRoulette>, args: CancelRouletteArgs) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Validate reason length
    require!(
        args.reason.len() <= 200,
        RouletteError::InvalidBetNumbers // Reusing error for simplicity
    );
    
    // Store refund amount
    let total_refunds = roulette.total_bet_amount;
    
    // TODO: Add refund transfer logic using CPI
    // This is commented out for IDL generation
    // transfer(transfer_ctx, total_refunds)?;
    
    // Update roulette state
    roulette.state = RouletteState::Cancelled;
    // roulette.updated_at = clock.unix_timestamp;
    
    // Emit roulette cancelled event
    emit!(RouletteCancelled {
        roulette_id: roulette.key(),
        reason: args.reason,
        total_refunds,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Roulette cancelled: {}, refunds: {}", roulette.key(), total_refunds);
    
    Ok(())
}