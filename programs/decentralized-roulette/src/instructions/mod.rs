pub mod initialize;
pub mod create_roulette;
pub mod place_bet;
pub mod lock_betting;
// VRF modules for production-ready verifiable randomness
pub mod spin_roulette;
pub mod settle_randomness;
pub mod vrf_client; // Legacy VRF implementation
pub mod switchboard_vrf_simple; // Simplified Switchboard VRF integration for testing
pub mod claim_winnings;
pub mod cancel_roulette;
pub mod process_game_lifecycle;
pub mod create_next_game;
pub mod process_automation;
// pub mod public_lifecycle_keeper; // Disabled due to borrowing issues
pub mod simple_lifecycle_keeper;
pub mod emergency_pause;
// pub mod thread_automation;
// pub mod create_automation_thread;

// Export account structs and specific functions with aliases to avoid conflicts
pub use initialize::{Initialize, handler as initialize_handler};
pub use create_roulette::{CreateRoulette, handler as create_roulette_handler};
pub use place_bet::{PlaceBet, handler as place_bet_handler};
pub use lock_betting::{LockBetting, handler as lock_betting_handler};
// VRF exports for production-ready verifiable randomness
pub use spin_roulette::{SpinRoulette, handler as spin_roulette_handler};
pub use settle_randomness::{SettleRandomness, handler as settle_randomness_handler};
pub use vrf_client::{InitializeVrfClient, RequestRandomness, ConsumeRandomness, handler as initialize_vrf_client_handler}; // Legacy VRF implementation
pub use switchboard_vrf_simple::{SimpleSwitchboardVrf, InitializeSwitchboardVrf, RequestSwitchboardRandomness, ConsumeSwitchboardRandomness, handler as simple_switchboard_vrf_handler}; // Simplified Switchboard VRF integration for testing
pub use claim_winnings::{ClaimWinnings, handler as claim_winnings_handler};
pub use cancel_roulette::{CancelRoulette, handler as cancel_roulette_handler};
pub use process_game_lifecycle::{ProcessGameLifecycle, handler as process_game_lifecycle_handler};
pub use create_next_game::{CreateNextGame, handler as create_next_game_handler};
pub use process_automation::{ProcessAutomation, handler as process_automation_handler};
// pub use public_lifecycle_keeper::*; // Disabled due to borrowing issues
pub use simple_lifecycle_keeper::{SimpleLifecycleKeeper, handler as simple_lifecycle_keeper_handler};
pub use emergency_pause::{EmergencyPauseToggle, ForceEndGame, handler as emergency_pause_handler, force_end_game_handler};
// pub use thread_automation::*;
// pub use create_automation_thread::*;