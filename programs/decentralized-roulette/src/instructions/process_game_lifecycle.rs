use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::{BettingLocked, RouletteExpired, RouletteSpinStarted, RouletteSpun};
use crate::errors::RouletteError;
use crate::utils;

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
                
                // Calculate total refunds needed
                let total_refunds = utils::calculate_total_refunds(&roulette)?;
                
                emit!(RouletteExpired {
                    roulette_id: roulette.key(),
                    reason: "Game expired".to_string(),
                    total_refunds,
                    timestamp: current_time,
                });
                
                msg!("Auto-expired roulette: {} (endTime passed) - Total refunds needed: {}", 
                     roulette.key(), total_refunds);
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
                    
                    // Calculate fees with overflow protection
                    let total_bet_amount = roulette.total_bet_amount;
                    let treasury_fee = total_bet_amount
                        .checked_mul(200u64)
                        .and_then(|result| result.checked_div(10000))
                        .ok_or(RouletteError::TreasuryFeeOverflow)?; // 2% treasury fee
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
                
                // Calculate total refunds needed
                let total_refunds = utils::calculate_total_refunds(&roulette)?;
                
                emit!(RouletteExpired {
                    roulette_id: roulette.key(),
                    reason: "Game expired".to_string(),
                    total_refunds,
                    timestamp: current_time,
                });
                
                msg!("Auto-expired roulette: {} (endTime passed) - Total refunds needed: {}", 
                     roulette.key(), total_refunds);
            }
            // Check if it's time to spin
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
                
                // Complete the game immediately
                roulette.winning_number = Some(winning_number);
                roulette.randomness_fulfilled = true;
                roulette.state = RouletteState::Completed;
                roulette.is_settled = true;
                roulette.completed_at = Some(current_time);
                
                // Calculate fees with overflow protection
                let total_bet_amount = roulette.total_bet_amount;
                let treasury_fee = total_bet_amount
                    .checked_mul(200u64)
                    .and_then(|result| result.checked_div(10000))
                    .ok_or(RouletteError::TreasuryFeeOverflow)?; // 2% treasury fee
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
                
                msg!("Auto-spun and completed roulette: {} with winning number: {}", 
                     roulette.key(), winning_number);
            }
        },
        RouletteState::Spinning => {
            // Check if game has expired
            if current_time >= roulette.end_time {
                roulette.state = RouletteState::Expired;
                roulette.completed_at = Some(current_time);
                
                // Calculate total refunds needed
                let total_refunds = utils::calculate_total_refunds(&roulette)?;
                
                emit!(RouletteExpired {
                    roulette_id: roulette.key(),
                    reason: "Game expired during spinning".to_string(),
                    total_refunds,
                    timestamp: current_time,
                });
                
                msg!("Auto-expired roulette: {} (expired during spinning) - Total refunds needed: {}", 
                     roulette.key(), total_refunds);
            }
            // Check if spinning should complete (simulate spinning duration)
            else if current_time >= roulette.spin_time + 30 { // 30 seconds spinning time
                // Generate winning number if not already set
                let winning_number = if let Some(number) = roulette.winning_number {
                    number
                } else {
                    // Fallback random generation
                    let random_seed = current_time.wrapping_add(clock.slot as i64);
                    match roulette.roulette_type {
                        crate::state::roulette::RouletteType::European => {
                            let result = random_seed.abs() % (EUROPEAN_ROULETTE_NUMBERS as i64);
                            result as u8
                        },
                        crate::state::roulette::RouletteType::American => {
                            let result = random_seed.abs() % (AMERICAN_ROULETTE_NUMBERS as i64);
                            result as u8
                        },
                    }
                };
                
                roulette.winning_number = Some(winning_number);
                roulette.randomness_fulfilled = true;
                roulette.state = RouletteState::Completed;
                roulette.is_settled = true;
                roulette.completed_at = Some(current_time);
                
                // Calculate fees with overflow protection
                let total_bet_amount = roulette.total_bet_amount;
                let treasury_fee = total_bet_amount
                    .checked_mul(200u64)
                    .and_then(|result| result.checked_div(10000))
                    .ok_or(RouletteError::TreasuryFeeOverflow)?; // 2% treasury fee
                roulette.treasury_fee_collected = treasury_fee;
                
                // Emit spin started event
                emit!(RouletteSpinStarted {
                    roulette_id: roulette.key(),
                    vrf_client: roulette.key(),
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
                
                msg!("Spinning completed for roulette: {} with winning number: {}", 
                     roulette.key(), winning_number);
            }
        },
        RouletteState::AwaitingRandomness => {
            // Check if game has expired
            if current_time >= roulette.end_time {
                roulette.state = RouletteState::Expired;
                roulette.completed_at = Some(current_time);
                
                // Calculate total refunds needed
                let total_refunds = utils::calculate_total_refunds(&roulette)?;
                
                emit!(RouletteExpired {
                    roulette_id: roulette.key(),
                    reason: "Game expired while awaiting randomness".to_string(),
                    total_refunds,
                    timestamp: current_time,
                });
                
                msg!("Auto-expired roulette: {} (expired awaiting randomness) - Total refunds needed: {}", 
                     roulette.key(), total_refunds);
            }
            // Check if randomness should be consumed (simulate VRF delay)
            else if let Some(request_time) = roulette.vrf_request_timestamp {
                if current_time >= request_time + VRF_REQUEST_DELAY {
                    // Auto-consume randomness (simulate VRF callback)
                    if let Some(vrf_client_key) = roulette.vrf_client {
                        // Generate winning number using VRF simulation
                        let randomness = utils::calculate_winning_number(
                            &[clock.unix_timestamp.to_le_bytes(), clock.slot.to_le_bytes()].concat().try_into().unwrap_or_default(),
                            match roulette.roulette_type {
                                crate::state::roulette::RouletteType::European => EUROPEAN_ROULETTE_NUMBERS,
                                crate::state::roulette::RouletteType::American => AMERICAN_ROULETTE_NUMBERS,
                            }
                        );
                        
                        roulette.winning_number = Some(randomness);
                        roulette.randomness_fulfilled = true;
                        roulette.state = RouletteState::Completed;
                        roulette.is_settled = true;
                        roulette.completed_at = Some(current_time);
                        
                        // Calculate fees
                        let total_bet_amount = roulette.total_bet_amount;
                        let treasury_fee = total_bet_amount
                            .checked_mul(200u64)
                            .and_then(|result| result.checked_div(10000))
                            .ok_or(RouletteError::TreasuryFeeOverflow)?;
                        roulette.treasury_fee_collected = treasury_fee;
                        
                        // Emit game completed event
                        emit!(RouletteSpun {
                            roulette_id: roulette.key(),
                            winning_number: randomness,
                            total_winners: 0,
                            total_payouts: 0,
                            house_edge_collected: 0,
                            treasury_fee_collected: treasury_fee,
                            timestamp: current_time,
                        });
                        
                        msg!("Randomness consumed for roulette: {} with winning number: {}", 
                             roulette.key(), randomness);
                    }
                }
            }
        },
        RouletteState::Completed => {
            // Game is already completed, nothing to do
            msg!("Roulette {} is already completed", roulette.key());
        },
        RouletteState::Expired => {
            // Game is already expired, nothing to do
            msg!("Roulette {} is already expired", roulette.key());
        },
        RouletteState::Cancelled => {
            // Game is cancelled, nothing to do
            msg!("Roulette {} is cancelled", roulette.key());
        },
        RouletteState::Created => {
            // Game hasn't started yet
            msg!("Roulette {} is created but not started", roulette.key());
        },
    }
    
    Ok(())
}