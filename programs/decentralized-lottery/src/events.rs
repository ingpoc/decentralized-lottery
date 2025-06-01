// src/events.rs
use anchor_lang::prelude::*;
use crate::state::lottery::LotteryState;

#[event]
/// Event emitted when a new lottery is created.
/// - Purpose: Signals the initialization of a new lottery with its configuration details.
/// - Context: Triggered by the `create_lottery` instruction after successfully setting up a lottery account.
pub struct LotteryCreated {
    pub lottery_id: Pubkey,
    pub lottery_type: String,
    pub ticket_price: u64,
    pub draw_time: i64,
    pub target_prize_pool: u64,
}

#[event]
/// Event emitted when a ticket is purchased for a lottery.
/// - Purpose: Records the purchase details, linking a buyer to a specific ticket in a lottery.
/// - Context: Triggered by the `buy_ticket` instruction upon successful token transfer and ticket account creation.
pub struct TicketPurchased {
    pub lottery_id: Pubkey,
    pub ticket_id: u64,
    pub buyer: Pubkey,
    pub number_of_tickets: u64,
    pub total_cost: u64,
    pub timestamp: i64,
}

#[event]
/// Event emitted when a lottery's state changes.
/// - Purpose: Tracks the lifecycle progression of a lottery through its various states (e.g., Created, Open, AwaitingRandomness, Completed, Expired).
/// - Context: Triggered by instructions like `transition_state` or `settle_randomness` when the lottery state is updated.
pub struct LotteryStateChanged {
    pub lottery_id: Pubkey,
    pub previous_state: LotteryState,
    pub new_state: LotteryState,
    pub timestamp: i64,
    pub total_tickets_sold: u64,
    pub current_prize_pool: u64,
}

#[event]
/// Event emitted when a winner is selected for a lottery.
/// - Purpose: Announces the winning ticket and the randomness value used to determine the winner, ensuring transparency.
/// - Context: Triggered by the `settle_randomness` instruction after verifying the oracle-provided randomness and calculating the winning ticket ID.
pub struct WinnerSelected {
    pub lottery_id: Pubkey,
    pub winning_ticket_id: u64,
    pub winning_ticket_pda: Pubkey,
    pub randomness_value: String, // Store as string for flexibility/size
    pub timestamp: i64,
}

#[event]
/// Event emitted when a prize is claimed by the winner.
/// - Purpose: Records the distribution of the prize pool, including any treasury fees deducted, to the winner.
/// - Context: Triggered by the `claim_prize` instruction after successfully transferring funds to the winner's token account.
pub struct PrizeClaimed {
    pub lottery_id: Pubkey,
    pub ticket_id: u64,
    pub winner: Pubkey,
    pub prize_pool: u64,
    pub treasury_fee: u64,
    pub winner_payout: u64,
    pub timestamp: i64,
}

#[event]
/// Event emitted when a winner is determined for a lottery.
/// - Purpose: Announces the winning ticket ID and randomness value used.
/// - Context: Triggered by the `select_winner` instruction after randomness is processed.
pub struct LotteryWinnerDetermined {
    pub lottery_id: Pubkey,
    pub previous_state: LotteryState,
    pub new_state: LotteryState,
    pub winner: Pubkey, // Placeholder, actual winner determined from ticket ID
    pub randomness: u64, // Can be the raw random value or derived winning ticket ID
    pub timestamp: i64,
}

#[event]
/// Event emitted when a ticket is refunded.
/// - Purpose: Indicates a refund has been issued for a ticket, typically when a lottery expires without a draw.
/// - Context: Triggered by the `claim_refund` instruction after returning the ticket cost to the buyer.
pub struct TicketRefunded {
    pub lottery_id: Pubkey,
    pub ticket_id: u64,
    pub buyer: Pubkey,
    pub refund_amount: u64,
    pub timestamp: i64,
}

#[event]
/// Event emitted when a VRF client is initialized for a lottery.
/// - Purpose: Signals the creation of a new VRF client for a specific lottery.
/// - Context: Triggered by the `init_vrf_client` instruction when setting up the VRF client state account.
pub struct VrfClientInitialized {
    pub lottery_id: Pubkey,
    pub vrf_client: Pubkey,
    pub vrf_account: Pubkey,
    pub timestamp: i64,
}

#[event]
/// Event emitted when randomness is requested from Switchboard.
/// - Purpose: Records when a lottery initiates a request for randomness from Switchboard VRF.
/// - Context: Triggered by the `request_randomness` instruction when making a CPI call to Switchboard.
pub struct RandomnessRequested {
    pub lottery_id: Pubkey,
    pub vrf_client: Pubkey,
    pub vrf_account: Pubkey,
    pub timestamp: i64,
}

#[event]
/// Event emitted when randomness is consumed from Switchboard.
/// - Purpose: Indicates that VRF randomness has been received and processed.
/// - Context: Triggered by the `consume_randomness` instruction after verifying and extracting the randomness.
pub struct RandomnessConsumed {
    pub lottery_id: Pubkey,
    pub vrf_client: Pubkey,
    pub timestamp: i64,
    pub dice_result: u64,
}

#[event]
/// Event emitted when a lottery drawing begins.
/// - Purpose: Signals the start of the lottery draw process.
/// - Context: Triggered when a lottery transitions to the Drawing state.
pub struct DrawingStarted {
    pub lottery_id: Pubkey,
    pub timestamp: i64,
    pub total_tickets: u64,
    pub prize_pool: u64,
    pub vrf_client: Option<Pubkey>,
}

#[event]
/// Event emitted when funds are withdrawn from the treasury.
/// - Purpose: Records treasury withdrawals for transparency and audit purposes.
/// - Context: Triggered by treasury withdrawal instructions when funds are transferred out.
pub struct TreasuryWithdrawal {
    pub treasury: Pubkey,
    pub amount: u64,
    pub destination: Pubkey,
    pub timestamp: i64,
    pub is_emergency: bool,
}
