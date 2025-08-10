use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::RouletteSpun;
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct SettleRandomness<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Spinning @ RouletteError::InvalidGameState,
        constraint = roulette.vrf_client.is_some() @ RouletteError::VrfClientNotInitialized
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Randomness account that contains the randomness
    /// CHECK: Switchboard randomness account
    #[account(
        constraint = randomness.key() == roulette.vrf_client.unwrap() @ RouletteError::InvalidVrfAccount
    )]
    pub randomness: UncheckedAccount<'info>,
    
    #[account(
        mut,
        constraint = roulette_token_account.mint == global_config.usdc_mint @ RouletteError::InvalidTokenAccount,
        seeds = [b"roulette_vault", roulette.key().as_ref()],
        bump
    )]
    pub roulette_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = treasury_token_account.mint == global_config.usdc_mint @ RouletteError::InvalidTokenAccount,
        constraint = treasury_token_account.key() == global_config.treasury_token_account @ RouletteError::InvalidTokenAccount
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    
    /// Anyone can call this instruction to settle the randomness
    pub caller: Signer<'info>,
}

pub fn handler(ctx: Context<SettleRandomness>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let global_config = &ctx.accounts.global_config;
    let clock = Clock::get()?;
    
    // Validate timing - must be at or after reveal time
    require!(
        clock.unix_timestamp >= roulette.reveal_time,
        RouletteError::TooEarly
    );
    
    // Since the spin_roulette already generated the winning number, just verify completion
    require!(
        roulette.randomness_fulfilled,
        RouletteError::RandomnessNotFulfilled
    );
    
    require!(
        roulette.winning_number.is_some(),
        RouletteError::RandomnessNotFulfilled
    );
    
    let winning_number = roulette.winning_number.unwrap();
    
    // Calculate payouts and fees with overflow protection
    let total_bet_amount = roulette.total_bet_amount;
    let treasury_fee = total_bet_amount
        .checked_mul(global_config.treasury_fee_percentage as u64)
        .and_then(|result| result.checked_div(10000))
        .ok_or(RouletteError::TreasuryFeeOverflow)?;
    
    // For roulette, house edge is built into the payout odds
    // Treasury fee is taken from total bets before payouts
    roulette.treasury_fee_collected = treasury_fee;
    
    // The remaining amount is available for payouts
    let _available_for_payouts = total_bet_amount
        .checked_sub(treasury_fee)
        .ok_or(RouletteError::ArithmeticOverflow)?;
    roulette.house_edge_collected = 0; // House edge is implicit in roulette odds
    
    // Transfer treasury fee if there's any to collect
    if treasury_fee > 0 {
        // Verify the roulette has sufficient funds for fee transfer
        require!(
            ctx.accounts.roulette_token_account.amount >= treasury_fee,
            RouletteError::InsufficientFunds
        );
        
        // Create PDA signer seeds for the roulette vault account
        let roulette_key = roulette.key();
        let roulette_seeds = &[
            b"roulette_vault",
            roulette_key.as_ref(),
            &[ctx.bumps.roulette_token_account]
        ];
        let signer_seeds = &[&roulette_seeds[..]];
        
        // Transfer treasury fee from roulette vault to treasury
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.roulette_token_account.to_account_info(),
                to: ctx.accounts.treasury_token_account.to_account_info(),
                authority: ctx.accounts.roulette_token_account.to_account_info(),
            },
            signer_seeds
        );
        
        token::transfer(transfer_ctx, treasury_fee)?;
        msg!("Treasury fee transferred: {} USDC", treasury_fee);
    }
    
    // Transition to completed state
    roulette.state = RouletteState::Completed;
    roulette.is_settled = true;
    roulette.completed_at = Some(clock.unix_timestamp);
    
    // Emit roulette spun event
    emit!(RouletteSpun {
        roulette_id: roulette.key(),
        winning_number,
        total_winners: 0, // Will be calculated when bets are processed
        total_payouts: 0, // Will be updated as payouts are claimed
        house_edge_collected: 0,
        treasury_fee_collected: treasury_fee,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Roulette settled: {} won, treasury fee: {}", 
         winning_number, treasury_fee);
    
    Ok(())
}

