use anchor_lang::prelude::*;
use switchboard_v2::VrfAccountData;

// Constants for account sizes
pub const VRF_RESULT_BUFFER_SIZE: usize = 32; // 32 bytes for randomness result

#[account]
#[derive(Debug)]
pub struct VrfClientState {
    // Bump seed for PDA derivation
    pub bump: u8,
    
    // Buffer to store randomness result (32 bytes)
    pub result_buffer: [u8; VRF_RESULT_BUFFER_SIZE],
    
    // Processed dice roll result
    pub dice_result: u64,
    
    // Timestamp of when the randomness was received
    pub timestamp: i64,
    
    // Switchboard VRF account pubkey
    pub vrf_account: Pubkey,
    
    // Associated lottery account
    pub lottery_account: Pubkey,
    
    // Escrow account pubkey (if needed for Switchboard interaction)
    pub escrow_account: Option<Pubkey>,
    
    // Flag to indicate if this randomness has been consumed
    pub is_consumed: bool,
}

impl VrfClientState {
    pub const ACCOUNT_SIZE: usize = 
        8 +                     // Discriminator
        1 +                     // bump
        VRF_RESULT_BUFFER_SIZE + // result_buffer (32 bytes)
        8 +                     // dice_result (u64)
        8 +                     // timestamp (i64)
        32 +                    // vrf_account (Pubkey)
        32 +                    // lottery_account (Pubkey)
        (1 + 32) +              // Option<Pubkey> escrow_account
        1;                      // is_consumed (bool)
}

// Helper functions to interact with Switchboard VRF
impl VrfClientState {
    // Get random u64 value from result buffer
    pub fn get_random_value(&self) -> u64 {
        let mut result_bytes = [0u8; 8];
        result_bytes.copy_from_slice(&self.result_buffer[0..8]);
        u64::from_le_bytes(result_bytes)
    }
    
    // Check if the VRF randomness is ready for consumption
    pub fn is_randomness_ready(&self, vrf_account_data: &VrfAccountData) -> bool {
        !self.is_consumed && vrf_account_data.has_callback_triggered()
    }
    
    // Mark randomness as consumed
    pub fn consume_randomness(&mut self) {
        self.is_consumed = true;
    }
}

