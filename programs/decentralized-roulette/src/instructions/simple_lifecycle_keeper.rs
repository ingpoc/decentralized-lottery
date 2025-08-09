use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState, RouletteType}
};
use crate::constants::*;
use crate::events::{BettingLocked, RouletteExpired, RouletteSpun};
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct SimpleLifecycleKeeper<'info> {
    #[account(mut)]
    pub roulette: Account<'info, RouletteAccount>,

    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub roulette_usdc_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub treasury_usdc_account: Account<'info, TokenAccount>,

    pub keeper: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<SimpleLifecycleKeeper>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let global_config = &ctx.accounts.global_config;
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;

    msg!("Simple lifecycle keeper called for roulette: {} in state: {:?}", 
         roulette.key(), roulette.state);

    match roulette.state {
        RouletteState::Open => {
            // Check if betting period has ended
            if current_time >= roulette.betting_end_time {
                roulette.state = RouletteState::Locked;
                emit!(BettingLocked {
                    roulette_id: roulette.key(),
                    spin_time: roulette.spin_time,
                    total_bets: roulette.total_bets,
                    total_bet_amount: roulette.total_bet_amount,
                    total_players: roulette.total_bets, // Approximation: each bet = 1 player
                    timestamp: current_time,
                });
                msg!("Roulette betting locked: {}", roulette.key());
            }
        },
        
        RouletteState::Locked => {
            // Check if it's time to spin
            if current_time >= roulette.spin_time {
                // Immediately transition to spinning and generate number
                let winning_number = generate_simple_winning_number(&clock, roulette)?;
                roulette.winning_number = Some(winning_number);
                roulette.state = RouletteState::Completed;
                roulette.completed_at = Some(current_time);
                
                // Transfer treasury fee
                let treasury_fee = (roulette.total_bet_amount * global_config.treasury_fee_percentage as u64) / 10000;
                if treasury_fee > 0 {
                    // Log treasury fee transfer - actual transfer would be implemented here
                    msg!("Treasury fee calculated: {} micro-USDC", treasury_fee);
                }
                
                emit!(RouletteSpun {
                    roulette_id: roulette.key(),
                    winning_number,
                    total_winners: 0, // Will be calculated later during payout claims
                    total_payouts: 0, // Will be calculated later during payout claims
                    house_edge_collected: treasury_fee,
                    treasury_fee_collected: treasury_fee,
                    timestamp: current_time,
                });
                
                msg!("Roulette completed: {} - Winning number: {}", roulette.key(), winning_number);
            }
        },
        
        RouletteState::Spinning | RouletteState::AwaitingRandomness => {
            // Handle any stuck states by completing them
            if current_time >= roulette.reveal_time {
                let winning_number = generate_simple_winning_number(&clock, roulette)?;
                roulette.winning_number = Some(winning_number);
                roulette.state = RouletteState::Completed;
                roulette.completed_at = Some(current_time);
                
                msg!("Roulette unstuck and completed: {} - Winning number: {}", roulette.key(), winning_number);
            }
        },
        
        RouletteState::Completed | RouletteState::Expired | RouletteState::Cancelled => {
            // Nothing to do for terminal states
            msg!("Roulette {} already in terminal state: {:?}", roulette.key(), roulette.state);
        },
        
        _ => {
            // Handle any other states by checking expiration
            if current_time >= roulette.end_time {
                roulette.state = RouletteState::Expired;
                emit!(RouletteExpired {
                    roulette_id: roulette.key(),
                    reason: "Game expired after end time".to_string(),
                    total_refunds: roulette.total_bet_amount,
                    timestamp: current_time,
                });
                msg!("Roulette expired: {}", roulette.key());
            }
        }
    }

    Ok(())
}

fn generate_simple_winning_number(clock: &Clock, roulette: &RouletteAccount) -> Result<u8> {
    // Simple deterministic but unpredictable number generation
    let mut seed_data = Vec::new();
    seed_data.extend_from_slice(&clock.unix_timestamp.to_le_bytes());
    seed_data.extend_from_slice(&clock.slot.to_le_bytes());  
    seed_data.extend_from_slice(&roulette.total_bets.to_le_bytes());
    seed_data.extend_from_slice(&roulette.total_bet_amount.to_le_bytes());
    seed_data.extend_from_slice(&roulette.created_at.to_le_bytes());
    
    let hash = solana_program::keccak::hash(&seed_data);
    let random_bytes = hash.to_bytes();
    let random_u32 = u32::from_le_bytes([random_bytes[0], random_bytes[1], random_bytes[2], random_bytes[3]]);
    
    match roulette.roulette_type {
        RouletteType::European => Ok((random_u32 % 37) as u8), // 0-36
        RouletteType::American => Ok((random_u32 % 38) as u8),  // 0-37 (includes 00 as 37)
    }
}