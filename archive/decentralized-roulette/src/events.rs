use anchor_lang::prelude::*;
use crate::state::roulette::{RouletteType, RouletteState, BetType};

#[event]
pub struct RouletteCreated {
    pub roulette_id: Pubkey,
    pub roulette_type: RouletteType,
    pub authority: Pubkey,
    pub min_bet: u64,
    pub max_bet: u64,
    pub game_duration: i64,
    pub start_time: i64,
    pub betting_end_time: i64,
    pub spin_time: i64,
    pub end_time: i64,
    pub is_autonomous: bool,
    pub keeper: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct BetPlaced {
    pub roulette_id: Pubkey,
    pub bet_id: u64,
    pub bettor: Pubkey,
    pub bet_type: BetType,
    pub bet_amount: u64,
    pub bet_numbers: Vec<u8>,
    pub total_bets: u64,
    pub total_bet_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct BettingLocked {
    pub roulette_id: Pubkey,
    pub total_bets: u64,
    pub total_bet_amount: u64,
    pub total_players: u64,
    pub spin_time: i64,
    pub timestamp: i64,
}

#[event]
pub struct RouletteSpinStarted {
    pub roulette_id: Pubkey,
    pub vrf_client: Pubkey,
    pub vrf_request_key: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct RandomnessRequested {
    pub roulette_id: Pubkey,
    pub vrf_client: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct RouletteSpun {
    pub roulette_id: Pubkey,
    pub winning_number: u8,
    pub total_winners: u64,
    pub total_payouts: u64,
    pub house_edge_collected: u64,
    pub treasury_fee_collected: u64,
    pub timestamp: i64,
}

#[event]
pub struct VrfCompleted {
    pub roulette_id: Pubkey,
    pub winning_number: u8,
    pub randomness_source: String,
    pub timestamp: i64,
}

#[event]
pub struct WinningsClaimed {
    pub roulette_id: Pubkey,
    pub bet_id: u64,
    pub winner: Pubkey,
    pub payout_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct RouletteStateChanged {
    pub roulette_id: Pubkey,
    pub old_state: RouletteState,
    pub new_state: RouletteState,
    pub timestamp: i64,
}

#[event]
pub struct RouletteExpired {
    pub roulette_id: Pubkey,
    pub reason: String,
    pub total_refunds: u64,
    pub timestamp: i64,
}

#[event]
pub struct RouletteCancelled {
    pub roulette_id: Pubkey,
    pub reason: String,
    pub total_refunds: u64,
    pub timestamp: i64,
}

#[event]
pub struct PayoutClaimed {
    pub roulette_id: Pubkey,
    pub bet_id: u64,
    pub bettor: Pubkey,
    pub payout_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct EmergencyPause {
    pub authority: Pubkey,
    pub previous_state: bool,
    pub new_state: bool,
    pub timestamp: i64,
}

#[event]
pub struct NextGameCreationTriggered {
    pub triggered_by: Pubkey,
    pub timestamp: i64,
    pub reason: String,
}

#[event]
pub struct KeeperRewarded {
    pub keeper: Pubkey,
    pub reward_amount: u64,
    pub action: String,
    pub timestamp: i64,
}

#[event]
pub struct NextGameTriggered {
    pub triggered_by: Pubkey,
    pub next_nonce: u64,
    pub timestamp: i64,
    pub reason: String,
}