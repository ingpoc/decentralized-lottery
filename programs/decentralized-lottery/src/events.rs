// src/events.rs
use anchor_lang::prelude::*;

#[event]
pub struct LotteryCreated {
    pub lottery_id: Pubkey,
    pub lottery_type: String,
    pub ticket_price: u64,
    pub draw_time: i64,
    pub prize_pool: u64,
}

#[event]
pub struct TicketPurchased {
    pub lottery_id: Pubkey,
    pub buyer: Pubkey,
    pub number_of_tickets: u64,
    pub total_cost: u64,
    pub timestamp: i64,
}