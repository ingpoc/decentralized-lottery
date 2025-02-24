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
    Cancelled,
}

impl LotteryState {
    pub fn can_transition_to(&self, next_state: &LotteryState) -> bool {
        match self {
            LotteryState::Created => matches!(next_state, LotteryState::Open | LotteryState::Cancelled),
            LotteryState::Open => matches!(next_state, LotteryState::Drawing | LotteryState::Cancelled),
            LotteryState::Drawing => matches!(next_state, LotteryState::Completed | LotteryState::Expired | LotteryState::Cancelled),
            LotteryState::Completed => false, // Terminal state
            LotteryState::Expired => false,   // Terminal state
            LotteryState::Cancelled => false, // Terminal state
        }
    }

    pub fn can_cancel(&self) -> bool {
        matches!(self, 
            LotteryState::Created | 
            LotteryState::Open | 
            LotteryState::Drawing
        )
    }
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
    pub auto_transition: bool,    // For automatic state transitions
    pub last_ticket_id: u64,     // For tracking tickets
}