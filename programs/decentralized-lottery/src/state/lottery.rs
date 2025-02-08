//src/state/lottery.rs
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum LotteryType {
    Daily,
    Weekly,
    Monthly,
}

impl std::fmt::Display for LotteryType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            LotteryType::Daily => write!(f, "daily"),
            LotteryType::Weekly => write!(f, "weekly"),
            LotteryType::Monthly => write!(f, "monthly"),
        }
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, Debug)]
pub enum LotteryState {
    Created,
    Open,
    Drawing,
    Completed,
    Expired,
}

#[account]
pub struct LotteryAccount {
    pub lottery_type: LotteryType,
    pub ticket_price: u64,
    pub draw_time: i64,
    pub prize_pool: u64,
    pub total_tickets: u64,
    pub winning_numbers: Option<Vec<u8>>,
    pub state: LotteryState,
    pub created_by: Pubkey,
    pub global_config: Pubkey,
}