use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer as SplTransfer};
use crate::state::{bet::BetAccount, roulette::{RouletteAccount, RouletteState}};
use crate::errors::RouletteError;
use crate::events::PayoutClaimed;
use crate::constants::ROULETTE_SEED;

#[derive(Accounts)]
pub struct ClaimPayout<'info> {
    #[account(
        constraint = roulette.state == RouletteState::Completed @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, RouletteAccount>,

    #[account(mut)]
    pub bet: Account<'info, BetAccount>,

    #[account(mut)]
    pub bettor_usdc_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub roulette_usdc_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = bettor.key() == bet.bettor @ RouletteError::InvalidAuthority)]
    pub bettor: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClaimPayout>) -> Result<()> {
    let bet = &mut ctx.accounts.bet;
    require!(!bet.is_claimed, RouletteError::WinningsAlreadyClaimed);
    require!(bet.is_winner, RouletteError::NotAWinningBet);

    let payout = bet.calculate_payout();

    // Transfer payout (signed by roulette PDA)
    let seeds = &[ROULETTE_SEED, ctx.accounts.roulette.created_by.as_ref(), &[ctx.accounts.roulette.bump]];
    let signer_seeds: &[&[&[u8]]] = &[seeds];
    let transfer_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        SplTransfer {
            from: ctx.accounts.roulette_usdc_account.to_account_info(),
            to: ctx.accounts.bettor_usdc_account.to_account_info(),
            authority: ctx.accounts.roulette.to_account_info(),
        },
        signer_seeds,
    );
    anchor_spl::token::transfer(transfer_ctx, payout)?;

    bet.is_claimed = true;
    bet.claimed_at = Some(Clock::get()?.unix_timestamp);

    let roulette = &mut ctx.accounts.roulette;
    roulette.total_payouts += payout;

    emit!(PayoutClaimed {
        roulette_id: roulette.key(),
        bet_id: bet.bet_id,
        bettor: bet.bettor,
        payout_amount: payout,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}