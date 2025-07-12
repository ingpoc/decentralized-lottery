use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::{BettingLocked, RouletteExpired, RouletteSpinStarted, RouletteSpun};
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct ProcessGameLifecycle<'info> {
    #[account(mut)]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Anyone can call this instruction
    pub caller: Signer<'info>,
}

pub fn handler(ctx: Context<ProcessGameLifecycle>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    let current_time = clock.unix_timestamp;
    
    msg!("Manual lifecycle processing for roulette: {}", roulette.key());
    msg!("Current time: {}, State: {:?}", current_time, roulette.state);
    msg!("Betting end time: {}, Spin time: {}, End time: {}", 
         roulette.betting_end_time, roulette.spin_time, roulette.end_time);
    
    match roulette.state {
        RouletteState::Open => {
            // Check if game has expired (endTime passed) - highest priority
            if current_time >= roulette.end_time {
                roulette.state = RouletteState::Expired;
                roulette.completed_at = Some(current_time);
                
                emit!(RouletteExpired {
                    roulette_id: roulette.key(),
                    reason: "Game expired".to_string(),
                    total_refunds: 0, // TODO: Implement refund logic
                    timestamp: current_time,
                });
                
                msg!("Auto-expired roulette: {} (endTime passed)", roulette.key());
            }
            // Auto-transition to Locked when betting period ends
            else if current_time >= roulette.betting_end_time {
                roulette.state = RouletteState::Locked;
                
                emit!(BettingLocked {
                    roulette_id: roulette.key(),
                    total_bets: roulette.total_bets,
                    total_bet_amount: roulette.total_bet_amount,
                    total_players: roulette.total_players,
                    spin_time: roulette.spin_time,
                    timestamp: current_time,
                });
                
                msg!("Auto-locked betting for roulette: {}", roulette.key());
                
                // Continue processing to check if we should immediately spin
                // This handles cases where betting ended and spin time has also passed
                if current_time >= roulette.spin_time {
                    // Skip to spinning immediately
                    roulette.state = RouletteState::Spinning;
                    
                    // Generate pseudo-random winning number using clock and slot
                    let random_seed = current_time.wrapping_add(clock.slot as i64);
                    let winning_number = match roulette.roulette_type {
                        crate::state::roulette::RouletteType::European => {
                            let result = random_seed.abs() % (EUROPEAN_ROULETTE_NUMBERS as i64);
                            result as u8
                        },
                        crate::state::roulette::RouletteType::American => {
                            let result = random_seed.abs() % (AMERICAN_ROULETTE_NUMBERS as i64);
                            result as u8
                        },
                    };
                    
                    // Complete the game immediately
                    roulette.winning_number = Some(winning_number);
                    roulette.randomness_fulfilled = true;
                    roulette.state = RouletteState::Completed;
                    roulette.is_settled = true;
                    roulette.completed_at = Some(current_time);
                    
                    // Calculate fees
                    let total_bet_amount = roulette.total_bet_amount;
                    let treasury_fee = (total_bet_amount * 200u64) / 10000; // 2% treasury fee
                    roulette.treasury_fee_collected = treasury_fee;
                    
                    // Emit spin started event
                    emit!(RouletteSpinStarted {
                        roulette_id: roulette.key(),
                        vrf_client: roulette.key(), // Use roulette as VRF client
                        vrf_request_key: roulette.key(),
                        timestamp: current_time,
                    });
                    
                    // Emit game completed event
                    emit!(RouletteSpun {
                        roulette_id: roulette.key(),
                        winning_number,
                        total_winners: 0, // Will be calculated when payouts are claimed
                        total_payouts: 0,
                        house_edge_collected: 0,
                        treasury_fee_collected: treasury_fee,
                        timestamp: current_time,
                    });
                    
                    msg!("Auto-completed roulette: {} with winning number: {}", 
                         roulette.key(), winning_number);
                }
            }
        },
        RouletteState::Locked => {
            // Check if game has expired (endTime passed) - highest priority
            if current_time >= roulette.end_time {
                roulette.state = RouletteState::Expired;
                roulette.completed_at = Some(current_time);
                
                emit!(RouletteExpired {
                    roulette_id: roulette.key(),
                    reason: "Game expired".to_string(),
                    total_refunds: 0, // TODO: Implement refund logic
                    timestamp: current_time,
                });
                
                msg!("Auto-expired roulette: {} (endTime passed)", roulette.key());
            }
            // Auto-transition to Spinning and generate result when spin time arrives
            else if current_time >= roulette.spin_time {
                roulette.state = RouletteState::Spinning;
                
                // Generate pseudo-random winning number using clock and slot
                let random_seed = current_time.wrapping_add(clock.slot as i64);
                let winning_number = match roulette.roulette_type {
                    crate::state::roulette::RouletteType::European => {
                        let result = random_seed.abs() % (EUROPEAN_ROULETTE_NUMBERS as i64);
                        result as u8
                    },
                    crate::state::roulette::RouletteType::American => {
                        let result = random_seed.abs() % (AMERICAN_ROULETTE_NUMBERS as i64);
                        result as u8
                    },
                };
                
                // Set winning number and complete the game
                roulette.winning_number = Some(winning_number);
                roulette.randomness_fulfilled = true;
                roulette.state = RouletteState::Completed;
                roulette.is_settled = true;
                roulette.completed_at = Some(current_time);
                
                // Calculate fees
                let total_bet_amount = roulette.total_bet_amount;
                let treasury_fee = (total_bet_amount * 200u64) / 10000; // 2% treasury fee
                roulette.treasury_fee_collected = treasury_fee;
                
                // Emit spin started event
                emit!(RouletteSpinStarted {
                    roulette_id: roulette.key(),
                    vrf_client: roulette.key(), // Use roulette as VRF client
                    vrf_request_key: roulette.key(),
                    timestamp: current_time,
                });
                
                // Emit game completed event
                emit!(RouletteSpun {
                    roulette_id: roulette.key(),
                    winning_number,
                    total_winners: 0, // Will be calculated when payouts are claimed
                    total_payouts: 0,
                    house_edge_collected: 0,
                    treasury_fee_collected: treasury_fee,
                    timestamp: current_time,
                });
                
                msg!("Auto-completed roulette: {} with winning number: {}", 
                     roulette.key(), winning_number);
            }
        },
        RouletteState::Spinning | RouletteState::AwaitingRandomness => {
            // Handle games that are stuck in spinning state
            msg!("Game is spinning - completing immediately");
            
            // Generate pseudo-random winning number using clock and slot
            let random_seed = current_time.wrapping_add(clock.slot as i64);
            let winning_number = match roulette.roulette_type {
                crate::state::roulette::RouletteType::European => {
                    let result = random_seed.abs() % (EUROPEAN_ROULETTE_NUMBERS as i64);
                    result as u8
                },
                crate::state::roulette::RouletteType::American => {
                    let result = random_seed.abs() % (AMERICAN_ROULETTE_NUMBERS as i64);
                    result as u8
                },
            };
            
            // Complete the game immediately
            roulette.winning_number = Some(winning_number);
            roulette.randomness_fulfilled = true;
            roulette.state = RouletteState::Completed;
            roulette.is_settled = true;
            roulette.completed_at = Some(current_time);
            
            // Calculate fees
            let total_bet_amount = roulette.total_bet_amount;
            let treasury_fee = (total_bet_amount * 200u64) / 10000; // 2% treasury fee
            roulette.treasury_fee_collected = treasury_fee;
            
            // Emit game completed event
            emit!(RouletteSpun {
                roulette_id: roulette.key(),
                winning_number,
                total_winners: 0, // Will be calculated when payouts are claimed
                total_payouts: 0,
                house_edge_collected: 0,
                treasury_fee_collected: treasury_fee,
                timestamp: current_time,
            });
            
            msg!("Manually completed roulette: {} with winning number: {}", 
                 roulette.key(), winning_number);
        },
        RouletteState::Completed => {
            msg!("Game is already completed");
        },
        RouletteState::Expired => {
            msg!("Game is already expired");
        },
        RouletteState::Cancelled => {
            msg!("Game is cancelled");
        },
        _ => {
            // No automatic transitions for terminal states
            msg!("No processing needed for state: {:?}", roulette.state);
        }
    }
    
    msg!("Manual lifecycle processing complete. New state: {:?}", roulette.state);
    
    Ok(())
}