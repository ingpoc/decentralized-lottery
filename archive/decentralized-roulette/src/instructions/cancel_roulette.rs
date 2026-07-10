use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer as SplTransfer};
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
        constraint = roulette.state != RouletteState::Completed && roulette.state != RouletteState::Cancelled @ RouletteError::InvalidGameState,
        close = authority
    )]
    pub roulette: Account<'info, RouletteAccount>,

    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = authority.key() == global_config.authority @ RouletteError::InvalidAuthority
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut)]
    pub roulette_usdc_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub treasury_usdc_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct CancelRouletteArgs {
    pub reason: String,
}

pub fn handler(ctx: Context<CancelRoulette>, args: CancelRouletteArgs) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;

    require!(args.reason.len() <= 200, RouletteError::InvalidBetNumbers);

    let total_refunds = roulette.total_bet_amount;

    // Transfer refunds back (simplified: to treasury; in prod, per bettor claim)
    let seeds = &[ROULETTE_SEED, roulette.created_by.as_ref(), &[roulette.bump]];
    let signer_seeds: &[&[&[u8]]] = &[seeds];
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        SplTransfer {
            from: ctx.accounts.roulette_usdc_account.to_account_info(),
            to: ctx.accounts.treasury_usdc_account.to_account_info(),  // Or bettor ATAs
            authority: ctx.accounts.roulette_usdc_account.to_account_info(),
        },
        signer_seeds,
    );
    anchor_spl::token::transfer(transfer_ctx, total_refunds)?;

    roulette.state = RouletteState::Cancelled;
    roulette.completed_at = Some(clock.unix_timestamp);

    emit!(RouletteCancelled {
        roulette_id: roulette.key(),
        reason: args.reason,
        total_refunds,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}