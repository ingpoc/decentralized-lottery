use anchor_lang::prelude::*;
use crate::state::roulette::BetType;

#[account]
pub struct BetAccount {
    pub roulette: Pubkey,
    pub bet_id: u64,
    pub bettor: Pubkey,
    pub bet_type: BetType,
    pub bet_amount: u64,
    pub bet_numbers: Vec<u8>,    // Numbers covered by this bet
    pub payout_multiplier: u16,  // Cached payout multiplier
    pub is_winner: bool,
    pub payout_amount: u64,
    pub is_claimed: bool,
    pub placed_at: i64,
    pub claimed_at: Option<i64>,
    pub bump: u8,
}

impl BetAccount {
    pub const ACCOUNT_SIZE: usize = 8 + // Discriminator
        32 +           // roulette Pubkey
        8 +            // bet_id u64
        32 +           // bettor Pubkey
        1 +            // bet_type enum (simplified to 1 byte)
        8 +            // bet_amount u64
        (4 + 38) +     // Vec<u8> bet_numbers (max 38 numbers for American roulette)
        2 +            // payout_multiplier u16
        1 +            // is_winner bool
        8 +            // payout_amount u64
        1 +            // is_claimed bool
        8 +            // placed_at i64
        (1 + 8) +      // Option<i64> claimed_at
        1;             // bump u8
    
    pub fn calculate_payout(&self) -> u64 {
        if self.is_winner {
            self.bet_amount * (self.payout_multiplier as u64 + 1)
        } else {
            0
        }
    }
    
    pub fn mark_as_winner(&mut self, winning_number: u8) {
        self.is_winner = self.bet_numbers.contains(&winning_number);
        if self.is_winner {
            self.payout_amount = self.calculate_payout();
        }
    }
}