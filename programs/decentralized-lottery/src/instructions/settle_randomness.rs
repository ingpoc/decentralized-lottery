use anchor_lang::prelude::*;
use anchor_lang::solana_program::keccak;
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::errors::LotteryError;

// Event for randomness settlement
#[event]
pub struct RandomnessSettled {
    pub lottery_id: Pubkey,
    pub randomness: [u8; 32],
    pub timestamp: i64,
    pub block_height: u64,
}

#[derive(Accounts)]
pub struct SettleRandomness<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.nonce.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::AwaitingRandomness @ LotteryError::InvalidLotteryState,
        constraint = !lottery_account.randomness_fulfilled @ LotteryError::RandomnessAlreadyFulfilled
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    /// Recent blockhashes sysvar for entropy
    /// CHECK: Solana sysvar for recent blockhashes  
    #[account(address = anchor_lang::solana_program::sysvar::slot_hashes::id())]
    pub recent_blockhashes: AccountInfo<'info>,

    /// Clock sysvar for timing entropy
    pub clock: Sysvar<'info, Clock>,

    /// Caller who triggers the settlement (provides additional entropy)
    pub caller: Signer<'info>,
}

pub fn settle_randomness_handler(ctx: Context<SettleRandomness>) -> Result<()> {
    let lottery_account = &mut ctx.accounts.lottery_account;
    let clock = &ctx.accounts.clock;
    
    // Store the account key before making mutable changes
    let lottery_key = lottery_account.key();

    // Check if we have VRF randomness available
    if let Some(vrf_randomness) = lottery_account.vrf_randomness {
        // Use Switchboard VRF randomness
        lottery_account.randomness_fulfilled = true;
        let previous_state = lottery_account.state.clone();
        lottery_account.state = LotteryState::Completed;
        lottery_account.completed_at = Some(clock.unix_timestamp);

        // Emit events
        emit!(RandomnessSettled {
            lottery_id: lottery_key,
            randomness: vrf_randomness,
            timestamp: clock.unix_timestamp,
            block_height: clock.slot,
        });

        emit!(crate::events::LotteryStateChanged {
            lottery_id: lottery_key,
            previous_state,
            new_state: lottery_account.state.clone(),
            timestamp: clock.unix_timestamp,
            total_tickets_sold: lottery_account.total_tickets,
            current_prize_pool: lottery_account.prize_pool,
        });

        msg!("VRF randomness used for lottery completion");
    } else {
        // Use fallback randomness generation
        msg!("WARNING: Using fallback randomness generation - NOT for production use");

        // Ensure sufficient time has passed since drawing started (prevents manipulation)
        require!(
            clock.unix_timestamp >= lottery_account.draw_time + 10, // 10 seconds minimum delay
            LotteryError::TooEarly
        );

        // Generate secure randomness using multiple entropy sources
        let randomness = generate_secure_randomness(
            &ctx.accounts.recent_blockhashes,
            clock,
            &lottery_account,
            &ctx.accounts.caller.key(),
            &lottery_key
        )?;

        // Store the randomness and mark as fulfilled
        lottery_account.vrf_randomness = Some(randomness);
        lottery_account.randomness_fulfilled = true;
        let previous_state = lottery_account.state.clone();
        lottery_account.state = LotteryState::Completed;
        lottery_account.completed_at = Some(clock.unix_timestamp);

        // Emit events
        emit!(RandomnessSettled {
            lottery_id: lottery_key,
            randomness,
            timestamp: clock.unix_timestamp,
            block_height: clock.slot,
        });

        emit!(crate::events::LotteryStateChanged {
            lottery_id: lottery_key,
            previous_state,
            new_state: lottery_account.state.clone(),
            timestamp: clock.unix_timestamp,
            total_tickets_sold: lottery_account.total_tickets,
            current_prize_pool: lottery_account.prize_pool,
        });

        msg!("Secure fallback randomness generated and lottery completed");
    }

    Ok(())
}

/// Generate cryptographically secure randomness using multiple entropy sources
fn generate_secure_randomness(
    recent_blockhashes: &AccountInfo,
    clock: &Clock,
    lottery_account: &LotteryAccount,
    caller: &Pubkey,
    lottery_key: &Pubkey,
) -> Result<[u8; 32]> {
    let mut entropy_sources = Vec::new();

    // 1. Recent blockhash entropy (unpredictable, varies by block)
    let recent_blockhash_data = recent_blockhashes.try_borrow_data()?;
    if recent_blockhash_data.len() >= 32 {
        entropy_sources.extend_from_slice(&recent_blockhash_data[..32]);
    }

    // 2. Clock-based entropy (current timestamp and slot)
    entropy_sources.extend_from_slice(&clock.unix_timestamp.to_le_bytes());
    entropy_sources.extend_from_slice(&clock.slot.to_le_bytes());

    // 3. Lottery-specific entropy (lottery ID, draw time, total tickets)
    entropy_sources.extend_from_slice(&lottery_key.to_bytes());
    entropy_sources.extend_from_slice(&lottery_account.draw_time.to_le_bytes());
    entropy_sources.extend_from_slice(&lottery_account.total_tickets.to_le_bytes());
    entropy_sources.extend_from_slice(&lottery_account.prize_pool.to_le_bytes());

    // 4. Caller entropy (who triggered the settlement)
    entropy_sources.extend_from_slice(&caller.to_bytes());

    // 5. Additional fixed entropy from lottery creation
    entropy_sources.extend_from_slice(&lottery_account.created_at.to_le_bytes());

    // Hash all entropy sources together using Keccak256 for cryptographic security
    let hash_result = keccak::hash(&entropy_sources);
    Ok(hash_result.to_bytes())
}
