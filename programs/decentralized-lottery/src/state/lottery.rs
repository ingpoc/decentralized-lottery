impl LotteryAccount {
    pub const ACCOUNT_SIZE: usize = 8 + // Discriminator
        1 +            // lottery_type enum (compact)
        8 +            // ticket_price u64
        8 +            // draw_time i64
        8 +            // prize_pool u64
        8 +            // total_tickets u64
        (1 + 32) +     // Option<Pubkey> winning_ticket
        1 +            // state enum (compact)
        32 +           // authority Pubkey
        8 +            // last_ticket_id u64
        (1 + 32) +     // Option<Pubkey> vrf_client
        (1 + 32) +     // Option<Pubkey> vrf_request_key
        (1 + 32) +     // Option<[u8; 32]> vrf_randomness
        1 +            // randomness_fulfilled bool
        1 +            // flags u8
        8 +            // created_at i64
        (1 + 8) +      // Option<i64> completed_at
        8;             // nonce u64
    // Flag bit masks
    const AUTO_TRANSITION_FLAG: u8 = 1 << 0;
    const PRIZE_POOL_LOCKED_FLAG: u8 = 1 << 1;
    const IS_CLAIMED_FLAG: u8 = 1 << 2;
    
    // Flag helper methods
    pub fn get_auto_transition(&self) -> bool {
        self.flags & Self::AUTO_TRANSITION_FLAG != 0
    }
    
    pub fn set_auto_transition(&mut self, value: bool) {
        if value {
            self.flags |= Self::AUTO_TRANSITION_FLAG;
        } else {
            self.flags &= !Self::AUTO_TRANSITION_FLAG;
        }
    }
    
    pub fn get_is_prize_pool_locked(&self) -> bool {
        self.flags & Self::PRIZE_POOL_LOCKED_FLAG != 0
    }
    
    pub fn set_is_prize_pool_locked(&mut self, value: bool) {
        if value {
            self.flags |= Self::PRIZE_POOL_LOCKED_FLAG;
        } else {
            self.flags &= !Self::PRIZE_POOL_LOCKED_FLAG;
        }
    }
    
    pub fn get_is_claimed(&self) -> bool {
        self.flags & Self::IS_CLAIMED_FLAG != 0
    }
    
    pub fn set_is_claimed(&mut self, value: bool) {
        if value {
            self.flags |= Self::IS_CLAIMED_FLAG;
        } else {
            self.flags &= !Self::IS_CLAIMED_FLAG;
        }
    }
    
    // VRF helper methods
    pub fn store_vrf_randomness(&mut self, randomness: [u8; 32]) {
        self.vrf_randomness = Some(randomness);
        self.randomness_fulfilled = true;
    }
    
    pub fn get_random_u64(&self) -> Option<u64> {
        self.vrf_randomness.map(|randomness| {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&randomness[0..8]);
            u64::from_le_bytes(bytes)
        })
    }
    
    pub fn select_winning_ticket(&self) -> Option<u64> {
        if self.total_tickets == 0 {
            return None;
        }
        
        self.get_random_u64().map(|random_value| {
            random_value % self.total_tickets
        })
    }
    
    // Helper methods for new fields
    pub fn mark_completed(&mut self, timestamp: i64) {
        self.completed_at = Some(timestamp);
        self.state = LotteryState::Completed;
    }
    
    pub fn is_completed(&self) -> bool {
        self.completed_at.is_some()
    }
    
    // Helper for is_prize_pool_locked via flags (already implemented above)
    pub fn is_prize_pool_locked(&self) -> bool {
        self.get_is_prize_pool_locked()
    }
    
    // Helper for auto_transition via flags
    pub fn auto_transition(&self) -> bool {
        self.get_auto_transition()
    }
}

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
    /// Ticket sales closed, lottery locked and ready for drawing
    Locked,
    /// Drawing in progress, VRF request initiated
    Drawing,
    /// Waiting for VRF callback with randomness
    AwaitingRandomness,
    /// Randomness received, winner determined
    Completed,
    /// Lottery expired without determining a winner
    Expired,
    /// Lottery manually cancelled
    Cancelled,
}

impl LotteryState {
    pub fn can_transition_to(&self, next_state: &LotteryState) -> bool {
        match self {
            LotteryState::Created => matches!(next_state, LotteryState::Open | LotteryState::Cancelled),
            LotteryState::Open => matches!(next_state, LotteryState::Locked | LotteryState::Cancelled),
            LotteryState::Locked => matches!(next_state, LotteryState::Drawing | LotteryState::Cancelled),
            LotteryState::Drawing => matches!(next_state, LotteryState::AwaitingRandomness | LotteryState::Expired | LotteryState::Cancelled),
            LotteryState::AwaitingRandomness => matches!(next_state, LotteryState::Completed | LotteryState::Expired | LotteryState::Cancelled),
            LotteryState::Completed => false, // Terminal state
            LotteryState::Expired => false,   // Terminal state
            LotteryState::Cancelled => false, // Terminal state
        }
    }

    pub fn can_cancel(&self) -> bool {
        matches!(self, 
            LotteryState::Created | 
            LotteryState::Open | 
            LotteryState::Locked |
            LotteryState::Drawing |
            LotteryState::AwaitingRandomness
        )
    }
    
    pub fn is_active(&self) -> bool {
        matches!(self,
            LotteryState::Created |
            LotteryState::Open |
            LotteryState::Locked |
            LotteryState::Drawing |
            LotteryState::AwaitingRandomness
        )
    }
    
    pub fn is_terminal(&self) -> bool {
        matches!(self,
            LotteryState::Completed |
            LotteryState::Expired |
            LotteryState::Cancelled
        )
    }
}

#[account]
pub struct LotteryAccount {
    // Core lottery data
    pub lottery_type: LotteryType,
    pub ticket_price: u64,
    pub draw_time: i64,
    pub prize_pool: u64,
    pub total_tickets: u64,
    pub winning_ticket: Option<Pubkey>,
    pub state: LotteryState,
    pub authority: Pubkey,
    pub last_ticket_id: u64,
    
    // VRF data
    pub vrf_client: Option<Pubkey>,
    pub vrf_request_key: Option<Pubkey>,
    pub vrf_randomness: Option<[u8; 32]>,
    pub randomness_fulfilled: bool,
    
    // Status flags packed into single byte
    pub flags: u8, // bit 0: auto_transition, bit 1: is_prize_pool_locked, bit 2: is_claimed
    
    // Metadata
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub nonce: u64,
}
