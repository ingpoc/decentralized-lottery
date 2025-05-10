use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct TicketAccount {
    // Lottery this ticket belongs to
    pub lottery: Pubkey,
    // The unique ID of this ticket within the lottery
    pub id: u64,
    // The pubkey of the account that purchased the ticket
    pub buyer: Pubkey,
    // State for refund tracking etc.
    pub is_claimed: bool, 
    // Bump seed for the PDA
    pub bump: u8,
}

impl TicketAccount {
    // Define size based on fields
    // 8 (discriminator) + 32 (lottery) + 8 (id) + 32 (buyer) + 1 (is_claimed) + 1 (bump)
    pub const ACCOUNT_SIZE: usize = 8 + 32 + 8 + 32 + 1 + 1;
} 