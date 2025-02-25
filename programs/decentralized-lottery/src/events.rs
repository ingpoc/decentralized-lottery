// src/events.rs
use anchor_lang::prelude::*;
use crate::state::lottery::LotteryState;

#[event]
pub struct LotteryCreated {
    pub lottery_id: Pubkey,
    pub lottery_type: String,
    pub ticket_price: u64,
    pub draw_time: i64,
    pub target_prize_pool: u64,
}

#[event]
pub struct TicketPurchased {
    pub lottery_id: Pubkey,
    pub buyer: Pubkey,
    pub number_of_tickets: u64,
    pub total_cost: u64,
    pub timestamp: i64,
}

#[event]
pub struct LotteryStateChanged {
    pub lottery_id: Pubkey,
    pub previous_state: LotteryState,
    pub new_state: LotteryState,
    pub timestamp: i64,
    pub total_tickets_sold: u64,
    pub current_prize_pool: u64,
}