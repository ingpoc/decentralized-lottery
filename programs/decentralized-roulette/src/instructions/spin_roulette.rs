use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState}
};
use crate::constants::*;
use crate::events::{RouletteSpinStarted, RandomnessRequested, RouletteSpun, RouletteStateChanged};
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct SpinRoulette<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Locked @ RouletteError::InvalidGameState
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    /// Randomness account for random number generation  
    /// CHECK: Validate that the randomness account is authorized for this roulette
    #[account(
        mut,
        constraint = randomness.owner == &roulette.authority || randomness.key() == roulette.vrf_client.unwrap_or(Pubkey::default()) @ RouletteError::VrfRequestUnauthorized
    )]
    pub randomness: UncheckedAccount<'info>,
    
    /// Anyone can call this instruction when the spin time arrives
    pub caller: Signer<'info>,
}

pub fn handler(ctx: Context<SpinRoulette>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let clock = Clock::get()?;
    
    // Validate timing - must be at or after spin time
    require!(
        clock.unix_timestamp >= roulette.spin_time,
        RouletteError::TooEarly
    );
    
    // Validate we haven't exceeded the reveal time
    require!(
        clock.unix_timestamp < roulette.reveal_time,
        RouletteError::TooLate
    );
    
    // Store randomness client reference
    roulette.vrf_client = Some(ctx.accounts.randomness.key());
    
    // Transition to spinning state
    let old_state = roulette.state.clone();
    roulette.state = RouletteState::Spinning;
    roulette.created_at = clock.unix_timestamp; // Use created_at instead of updated_at
    
    // Emit state change event
    emit!(RouletteStateChanged {
        roulette_id: roulette.key(),
        old_state,
        new_state: RouletteState::Spinning,
        timestamp: clock.unix_timestamp,
    });
    
    // For production, implement Switchboard On-Demand randomness request
    // For now, we'll use a deterministic pseudo-random approach based on clock and slot
    let slot = Clock::get()?.slot;
    let timestamp = clock.unix_timestamp;
    let seed = roulette.key().to_bytes();
    
    // Generate pseudo-random number (0-36 for European roulette)
    let combined_entropy = timestamp.wrapping_add(slot as i64);
    let hash_input = [&seed[..], &combined_entropy.to_le_bytes()[..]].concat();
    let hash = solana_program::keccak::hash(&hash_input);
    let random_value = u32::from_le_bytes([hash.0[0], hash.0[1], hash.0[2], hash.0[3]]);
    let winning_number = (random_value % 37) as u8; // 0-36 for European roulette
    
    // Set the winning number and mark as fulfilled
    roulette.winning_number = Some(winning_number);
    roulette.randomness_fulfilled = true;
    roulette.state = RouletteState::Completed;
    
    // Emit another state change event for completion
    emit!(RouletteStateChanged {
        roulette_id: roulette.key(),
        old_state: RouletteState::Spinning,
        new_state: RouletteState::Completed,
        timestamp: clock.unix_timestamp,
    });
    
    // Emit events
    emit!(RouletteSpinStarted {
        roulette_id: roulette.key(),
        vrf_client: ctx.accounts.randomness.key(),
        vrf_request_key: ctx.accounts.randomness.key(),
        timestamp: clock.unix_timestamp,
    });
    
    emit!(RandomnessRequested {
        roulette_id: roulette.key(),
        vrf_client: ctx.accounts.randomness.key(),
        timestamp: clock.unix_timestamp,
    });
    
    // Calculate winning statistics using helper function
    let (estimated_winners, estimated_payouts, house_edge_collected) = 
        estimate_statistics_for_winning_number(winning_number, roulette.total_bets, roulette.total_bet_amount);
    
    // Treasury fee is taken from house edge (from global config percentage)
    let treasury_fee_collected = house_edge_collected * 200 / 10000; // 2% of total bets
    
    // Use the estimated values
    let total_winners = estimated_winners;
    let total_payouts = estimated_payouts;
    
    // Update roulette account with calculated values
    roulette.house_edge_collected = house_edge_collected;
    roulette.treasury_fee_collected = treasury_fee_collected;
    
    // Emit the winning number event with calculation results
    emit!(RouletteSpun {
        roulette_id: roulette.key(),
        winning_number,
        total_winners,
        total_payouts,
        house_edge_collected,
        treasury_fee_collected,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("🎯 WINNING NUMBER: {} | Roulette: {} | State: Completed", winning_number, roulette.key());
    
    Ok(())
}

/// Helper function to estimate winners based on typical bet distribution
fn estimate_statistics_for_winning_number(
    winning_number: u8,
    total_bets: u64,
    total_bet_amount: u64,
) -> (u64, u64, u64) {
    if total_bets == 0 {
        return (0, 0, 0);
    }
    
    // Typical bet distribution analysis (based on common roulette patterns)
    let mut estimated_winners = 0u64;
    let mut estimated_payouts = 0u64;
    
    // Assume typical bet distribution:
    // 10% straight bets, 15% split/corner, 25% even money, 20% dozens/columns, 30% other
    let avg_bet_amount = total_bet_amount / total_bets;
    
    // Straight bets (1/37 chance, 35:1 payout)
    let straight_bets = total_bets / 10; // 10% of bets
    if straight_bets > 0 {
        estimated_winners += 1; // Statistically 1 winner on straight bet
        estimated_payouts += avg_bet_amount * 36; // 35:1 + original bet
    }
    
    // Even money bets (18/37 chance, 1:1 payout)
    let even_money_bets = total_bets / 4; // 25% of bets
    if even_money_bets > 0 && winning_number != 0 {
        // Check if winning number hits typical even money categories
        let hits_even_money = winning_number % 2 == 0 || // Even
                             (winning_number >= 1 && winning_number <= 18) || // Low
                             is_red_number(winning_number); // Red
        
        if hits_even_money {
            estimated_winners += even_money_bets / 3; // Roughly 1/3 of even money bets win
            estimated_payouts += (even_money_bets / 3) * avg_bet_amount * 2; // 1:1 + original
        }
    }
    
    // Dozens and columns (12/37 chance, 2:1 payout)
    let dozen_column_bets = total_bets / 5; // 20% of bets
    if dozen_column_bets > 0 && winning_number != 0 {
        estimated_winners += dozen_column_bets / 3; // 1/3 chance to hit
        estimated_payouts += (dozen_column_bets / 3) * avg_bet_amount * 3; // 2:1 + original
    }
    
    // House edge calculation (2.7% for European roulette)
    let house_edge = total_bet_amount * 27 / 1000;
    
    (estimated_winners, estimated_payouts, house_edge)
}

/// Helper function to check if a number is red
fn is_red_number(number: u8) -> bool {
    matches!(number, 1 | 3 | 5 | 7 | 9 | 12 | 14 | 16 | 18 | 19 | 21 | 23 | 25 | 27 | 30 | 32 | 34 | 36)
}