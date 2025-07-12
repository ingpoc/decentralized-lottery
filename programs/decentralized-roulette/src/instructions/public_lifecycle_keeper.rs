use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer as SplTransfer};
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState, RouletteType}
};
use crate::constants::*;
use crate::events::{BettingLocked, RouletteExpired, RouletteSpun};
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct PublicLifecycleKeeper<'info> {
    #[account(mut)]
    pub roulette: Account<'info, RouletteAccount>,

    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub roulette_usdc_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub treasury_usdc_account: Account<'info, TokenAccount>,

    // Public crank - anyone can call to advance game state
    #[account(mut)]
    pub crank_caller: Signer<'info>, // Incentivized crank caller

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<PublicLifecycleKeeper>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    match roulette.state {
        RouletteState::Open => {
            if current_time >= roulette.end_time {
                expire_roulette(roulette, current_time)?;
            } else if current_time >= roulette.betting_end_time {
                lock_betting(roulette, current_time)?;
                // If it's also time to spin, do it immediately
                if current_time >= roulette.spin_time {
                    spin_and_settle(ctx)?;
                }
            }
        },
        RouletteState::Locked => {
            if current_time >= roulette.end_time {
                expire_roulette(roulette, current_time)?;
            } else if current_time >= roulette.spin_time {
                spin_and_settle(ctx)?;
            }
        },
        RouletteState::Spinning => {
            // For immediate settlement
            complete_roulette_spin(ctx)?;
        },
        _ => {
            // No action needed for Completed, Expired, Cancelled states
        },
    }

    Ok(())
}

fn lock_betting(roulette: &mut Account<RouletteAccount>, current_time: i64) -> Result<()> {
    roulette.state = RouletteState::Locked;
    
    emit!(BettingLocked {
        roulette_id: roulette.key(),
        total_bets: roulette.total_bets,
        total_bet_amount: roulette.total_bet_amount,
        total_players: roulette.total_players,
        spin_time: roulette.spin_time,
        timestamp: current_time,
    });

    msg!("Betting locked for roulette: {}", roulette.key());
    Ok(())
}

fn expire_roulette(roulette: &mut Account<RouletteAccount>, current_time: i64) -> Result<()> {
    roulette.state = RouletteState::Expired;
    roulette.completed_at = Some(current_time);
    
    emit!(RouletteExpired {
        roulette_id: roulette.key(),
        reason: "Game duration exceeded".to_string(),
        total_refunds: roulette.total_bet_amount,
        timestamp: current_time,
    });

    msg!("Roulette expired: {}", roulette.key());
    Ok(())
}

fn spin_and_settle(ctx: Context<PublicLifecycleKeeper>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Set to spinning state briefly
    roulette.state = RouletteState::Spinning;
    
    // Generate pseudo-random winning number immediately
    let winning_number = generate_pseudo_random_number(&clock, &roulette.key(), roulette.roulette_type)?;
    
    // Complete the spin immediately
    complete_spin_with_number(ctx, winning_number)
}

fn complete_roulette_spin(ctx: Context<PublicLifecycleKeeper>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Generate winning number if not already set
    if roulette.winning_number.is_none() {
        let winning_number = generate_pseudo_random_number(&clock, &roulette.key(), roulette.roulette_type)?;
        complete_spin_with_number(ctx, winning_number)
    } else {
        // Already completed
        Ok(())
    }
}

fn complete_spin_with_number(ctx: Context<PublicLifecycleKeeper>, winning_number: u8) -> Result<()> {
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    // Calculate treasury fee first while we can still borrow
    let treasury_fee = {
        let roulette = &ctx.accounts.roulette;
        let global_config = &ctx.accounts.global_config;
        (roulette.total_bet_amount as u128 * global_config.treasury_fee_percentage as u128 / 10000) as u64
    };
    
    // Get roulette key for event emission
    let roulette_key = ctx.accounts.roulette.key();
    
    // Transfer treasury fee first
    transfer_treasury_fee(&ctx, treasury_fee)?;
    
    // Now update roulette state
    let roulette = &mut ctx.accounts.roulette;
    roulette.state = RouletteState::Completed;
    roulette.winning_number = Some(winning_number);
    roulette.randomness_fulfilled = true;
    roulette.completed_at = Some(current_time);
    
    emit!(RouletteSpun {
        roulette_id: roulette_key,
        winning_number,
        total_winners: 0, // Will be calculated when claims are made
        total_payouts: 0, // Will be calculated when claims are made
        house_edge_collected: 0, // Calculated based on bet types
        treasury_fee_collected: treasury_fee,
        timestamp: current_time,
    });

    msg!("Roulette completed: {} - Winning number: {}", roulette_key, winning_number);
    Ok(())
}

fn transfer_treasury_fee(ctx: &Context<PublicLifecycleKeeper>, treasury_fee: u64) -> Result<()> {
    if treasury_fee > 0 {
        let roulette = &ctx.accounts.roulette;
        let seeds = &[ROULETTE_SEED, roulette.created_by.as_ref(), &[roulette.bump]];
        let signer_seeds: &[&[&[u8]]] = &[seeds];
        
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            SplTransfer {
                from: ctx.accounts.roulette_usdc_account.to_account_info(),
                to: ctx.accounts.treasury_usdc_account.to_account_info(),
                authority: ctx.accounts.roulette.to_account_info(),
            },
            signer_seeds,
        );
        anchor_spl::token::transfer(transfer_ctx, treasury_fee)?;
    }
    Ok(())
}

/// Generate a pseudo-random number using on-chain entropy
/// This is deterministic but unpredictable at bet placement time
fn generate_pseudo_random_number(clock: &Clock, roulette_key: &Pubkey, roulette_type: RouletteType) -> Result<u8> {
    // Combine multiple entropy sources
    let mut seed_data = Vec::new();
    seed_data.extend_from_slice(&clock.unix_timestamp.to_le_bytes());
    seed_data.extend_from_slice(&clock.slot.to_le_bytes());
    seed_data.extend_from_slice(roulette_key.as_ref());
    
    // Simple hash-based pseudo-random generation
    let mut hash_input = [0u8; 32];
    let len = std::cmp::min(seed_data.len(), 32);
    hash_input[..len].copy_from_slice(&seed_data[..len]);
    
    // Use a simple XOR-based hash
    let mut result = 0u8;
    for &byte in &hash_input {
        result ^= byte;
        result = result.wrapping_mul(7).wrapping_add(1);
    }
    
    // Map to roulette range
    let max_number = match roulette_type {
        RouletteType::European => EUROPEAN_ROULETTE_NUMBERS, // 0-36 (37 numbers)
        RouletteType::American => AMERICAN_ROULETTE_NUMBERS,  // 0-36 + 00 (38 numbers) 
    };
    
    Ok(result % max_number)
}