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