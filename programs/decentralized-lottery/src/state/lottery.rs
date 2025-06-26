// Add account size constant and helper methods
impl LotteryAccount {
    pub const ACCOUNT_SIZE: usize = 8 + // Discriminator
        // Base fields
        (4 + 1) +      // lottery_type enum (1 byte + 4 for enum tag)
        8 +            // ticket_price u64
        8 +            // draw_time i64
        8 +            // prize_pool u64
        8 +            // total_tickets u64
        (1 + 32) +     // Option<Pubkey> winning_ticket
        (4 + 1) +      // state enum (1 byte + 4 for enum tag)
        32 +           // created_by Pubkey
        32 +           // global_config Pubkey
        1 +            // auto_transition bool
        8 +            // last_ticket_id u64
        32 +           // authority Pubkey
        
        // VRF-related fields
        (1 + 32) +     // Option<Pubkey> vrf_client
        (1 + 32) +     // Option<[u8; 32]> vrf_randomness
        (1 + 32) +     // Option<Pubkey> vrf_request_account
        (1 + 32) +     // Option<Pubkey> oracle_pubkey
        (1 + 32) +     // vrf_request_key: Option<Pubkey>
        1 +            // randomness_fulfilled: bool
        
        // Prize and state tracking
        1 +            // is_prize_pool_locked bool
        8 +            // target_prize_pool u64
        1 +            // is_claimed bool
        
        // Additional metadata
        8 +            // created_at i64
        (1 + 8) +      // Option<i64> completed_at
        8;             // nonce u64
    
    // Helper method to initialize VRF fields
    pub fn initialize_vrf(&mut self, vrf_client: Pubkey, oracle_pubkey: Option<Pubkey>, vrf_request_key: Pubkey) {
        self.vrf_client = Some(vrf_client);
        self.oracle_pubkey = oracle_pubkey; // Store if provided, could be None
        self.vrf_request_key = Some(vrf_request_key);
        self.randomness_fulfilled = false;
        self.vrf_randomness = None;
    }
    
    // Helper method to store VRF randomness
    pub fn store_vrf_randomness(&mut self, randomness: [u8; 32]) {
        self.vrf_randomness = Some(randomness);
    }
    
    // Helper method to get a random u64 from the stored randomness
    pub fn get_random_u64(&self) -> Option<u64> {
        self.vrf_randomness.map(|randomness| {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&randomness[0..8]);
            u64::from_le_bytes(bytes)
        })
    }
    
    // Helper method to select a winning ticket using the stored randomness
    pub fn select_winning_ticket(&self) -> Option<u64> {
        if self.total_tickets == 0 {
            return None;
        }
        
        self.get_random_u64().map(|random_value| {
            // Use modulo to select a ticket within the valid range
            random_value % self.total_tickets
        })
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
    pub lottery_type: LotteryType,
    pub ticket_price: u64,
    pub draw_time: i64,
    pub prize_pool: u64,
    pub total_tickets: u64,
    pub winning_ticket: Option<Pubkey>,
    pub state: LotteryState,
    pub created_by: Pubkey,
    pub global_config: Pubkey,
    pub auto_transition: bool,    // For automatic state transitions
    pub last_ticket_id: u64,      // For tracking tickets
    pub authority: Pubkey,        // Authority who can manage this lottery
    
    // VRF-related fields
    pub vrf_client: Option<Pubkey>,            // VRF client account PDA
    pub vrf_randomness: Option<[u8; 32]>,      // Raw randomness bytes from VRF
    pub vrf_request_account: Option<Pubkey>,   // Legacy field, can be removed or repurposed
    pub oracle_pubkey: Option<Pubkey>,         // Oracle/VRF account for randomness
    pub vrf_request_key: Option<Pubkey>,       // Stores the key of the VRF request account
    pub randomness_fulfilled: bool,            // Tracks if valid randomness has been received
    
    // Prize and state tracking
    pub is_prize_pool_locked: bool,  // To lock prize pool during drawing
    pub target_prize_pool: u64,      // Target prize pool amount (can be 0 if no target)
    pub is_claimed: bool,            // Whether the prize has been claimed
    
    // Additional metadata
    pub created_at: i64,             // Timestamp when lottery was created
    pub completed_at: Option<i64>,   // Timestamp when lottery completed/expired/cancelled
    pub nonce: u64,                  // Nonce used for PDA derivation
}
