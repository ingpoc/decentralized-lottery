use anchor_lang::prelude::*;
use switchboard_v2::VrfAccountData;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState, BetType},
    bet::BetAccount
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
    
    /// VRF account that contains the randomness
    #[account(
        constraint = vrf.key() == roulette.vrf_client.unwrap() @ RouletteError::InvalidVrfAccount
    )]
    pub vrf: AccountLoader<'info, VrfAccountData>,
    
    /// CHECK: Roulette token account (holds all bets)
    #[account(mut)]
    pub roulette_token_account: AccountInfo<'info>,
    
    /// CHECK: Treasury token account for collecting fees
    #[account(mut)]
    pub treasury_token_account: AccountInfo<'info>,
    
    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
    
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
    
    // Load VRF account and get randomness
    let vrf = ctx.accounts.vrf.load()?;
    require!(
        vrf.result.value.len() >= 32,
        RouletteError::RandomnessNotFulfilled
    );
    
    // Extract randomness and convert to winning number
    let randomness = vrf.result.value[..32].try_into().unwrap();
    let winning_number = calculate_winning_number(&randomness, &roulette.roulette_type);
    
    // Store randomness and winning number
    roulette.vrf_randomness = Some(randomness);
    roulette.vrf_randomness_fulfilled = true;
    roulette.winning_number = Some(winning_number);
    
    // Calculate payouts and fees
    let total_bet_amount = roulette.total_bet_amount;
    let treasury_fee = (total_bet_amount * global_config.treasury_fee_percentage as u64) / 10000;
    
    // For roulette, house edge is built into the payout odds
    // Treasury fee is taken from total bets before payouts
    roulette.treasury_fee_collected = treasury_fee;
    
    // The remaining amount is available for payouts
    let available_for_payouts = total_bet_amount - treasury_fee;
    roulette.house_edge_collected = 0; // House edge is implicit in roulette odds
    
    // TODO: Add treasury fee transfer logic using CPI
    // This is commented out for IDL generation
    // transfer(transfer_ctx, treasury_fee)?;
    
    // Transition to completed state
    roulette.state = RouletteState::Completed;
    roulette.is_settled = true;
    roulette.updated_at = clock.unix_timestamp;
    
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

fn calculate_winning_number(randomness: &[u8; 32], roulette_type: &crate::state::roulette::RouletteType) -> u8 {
    // Convert first 8 bytes of randomness to u64
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&randomness[..8]);
    let random_value = u64::from_le_bytes(bytes);
    
    // Calculate winning number based on roulette type
    match roulette_type {
        crate::state::roulette::RouletteType::European => {
            (random_value % EUROPEAN_ROULETTE_NUMBERS as u64) as u8
        },
        crate::state::roulette::RouletteType::American => {
            (random_value % AMERICAN_ROULETTE_NUMBERS as u64) as u8
        },
    }
}