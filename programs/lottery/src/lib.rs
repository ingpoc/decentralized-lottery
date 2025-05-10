use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// Replace with your actual program ID
declare_id!("Lottery1111111111111111111111111111111111111");

const LOTTERY_TOKEN_SEED: &[u8] = b"lottery_token";
const TICKET_SEED: &[u8] = b"ticket";

// Placeholder for Switchboard Program ID - Replace with actual ID
// declare_id!("SWITCHBOARD_PROGRAM_ID_PLACEHOLDER");
// pub mod switchboard_program { // Or however you import/use it
//     // ...
// }

#[program]
pub mod lottery {
    use super::*;

    // Initialize the Lottery
    pub fn initialize_lottery(ctx: Context<InitializeLottery>) -> Result<()> {
        let lottery = &mut ctx.accounts.lottery;
        let treasury_bump = *ctx.bumps.get("lottery_treasury").ok_or(ErrorCode::BumpError)?;

        lottery.admin = *ctx.accounts.admin.key;
        lottery.total_tickets = 0;
        lottery.winner_index = 0; // Use 0 as initial/sentinel value
        lottery.is_winner_selected = false;
        lottery.prize_mint = ctx.accounts.prize_mint.key();
        lottery.treasury_bump = treasury_bump;

        Ok(())
    }

    // Allows a user to buy a lottery ticket
    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        let ticket_account = &mut ctx.accounts.ticket_account;
        let lottery = &mut ctx.accounts.lottery;
        let buyer = &ctx.accounts.buyer;

        // Set ticket owner and assign ticket ID based on current total tickets
        ticket_account.owner = *buyer.key;
        ticket_account.ticket_id = lottery.total_tickets;

        // Increment total tickets sold *before* using it in seeds for the next ticket
        lottery.total_tickets = lottery.total_tickets.checked_add(1).ok_or(ErrorCode::Overflow)?;

        // Transfer ticket price from buyer to lottery treasury
        let ticket_price = 100_000; // Example: 0.1 USDC (assuming 6 decimals)

        let cpi_accounts = Transfer {
            from: ctx.accounts.buyer_token_account.to_account_info(),
            to: ctx.accounts.lottery_treasury.to_account_info(),
            authority: buyer.to_account_info(), // Buyer signs the transaction
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);

        token::transfer(cpi_ctx, ticket_price)?;

        Ok(())
    }

    // // Placeholder: Request randomness from Switchboard VRF
    // pub fn request_randomness(ctx: Context<RequestRandomness>) -> Result<()> {
    //     // Placeholder CPI call to Switchboard vrf_request_randomness
    //     // ... (rest of commented out function)
    //     msg!("Placeholder: Randomness requested from Switchboard.");
    //     Ok(())
    // }

    // // Placeholder: Handle Switchboard VRF callback
    // pub fn fulfill_randomness(ctx: Context<FulfillRandomness>, randomness_bytes: Vec<u8>) -> Result<()> {
    //     // Placeholder: Verify the VRF proof and caller
    //     // ... (rest of commented out function)
    //     msg!("Placeholder: Winner index set to {}", winning_index);
    //     Ok(())
    // }

    // Select the winner using VRF randomness output (or admin input for testing)
    // Note: For testing without VRF, admin can call this directly
    pub fn select_winner(ctx: Context<SelectWinner>, randomness: u64) -> Result<()> {
        let lottery = &mut ctx.accounts.lottery;
        if lottery.total_tickets == 0 {
            return Err(ErrorCode::NoTickets.into());
        }
        // Calculate the winning ticket index using the provided randomness
        let winning_index = randomness % (lottery.total_tickets as u64);
        lottery.winner_index = winning_index;
        lottery.is_winner_selected = true; // Add a flag to indicate selection
        Ok(())
    }

    // Allow the winning ticket owner to claim the prize
    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        let lottery = &ctx.accounts.lottery;
        let ticket = &ctx.accounts.ticket_account;

        // Verify that the claimant is the owner of the winning ticket
        require!(ticket.owner == *ctx.accounts.claimant.key, ErrorCode::Unauthorized);
        require!(ticket.ticket_id as u64 == lottery.winner_index, ErrorCode::NotWinner);
        require!(lottery.is_winner_selected, ErrorCode::WinnerNotSelected);
        // Optional: Add a check to prevent claiming multiple times if needed

        // Transfer the prize from the lottery treasury to the winner
        let prize_amount = 1_000_000; // Example: 1 USDC (assuming 6 decimals)

        let cpi_accounts = Transfer {
            from: ctx.accounts.lottery_treasury.to_account_info(),
            to: ctx.accounts.winner_token_account.to_account_info(),
            authority: ctx.accounts.lottery_treasury.to_account_info(), // The PDA treasury account itself is the authority
        };

        // Define seeds for PDA authority
        let lottery_key = lottery.key();
        let seeds = &[
            LOTTERY_TOKEN_SEED,
            lottery_key.as_ref(),
            &[lottery.treasury_bump]
        ];
        let signer = &[&seeds[..]];

        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);

        token::transfer(cpi_ctx, prize_amount)?;

        // Optional: Mark prize as claimed
        // ticket.is_claimed = true; // Need to add field to TicketAccount

        Ok(())
    }
}

// Context for InitializeLottery
#[derive(Accounts)]
pub struct InitializeLottery<'info> {
    #[account(init, payer = admin, space = 8 + Lottery::SIZE)] // 8 bytes for discriminator
    pub lottery: Account<'info, Lottery>,
    #[account(
        init,
        payer = admin,
        token::mint = prize_mint,
        token::authority = lottery_treasury, // PDA is the authority
        seeds = [LOTTERY_TOKEN_SEED, lottery.key().as_ref()],
        bump
    )]
    pub lottery_treasury: Account<'info, TokenAccount>,
    pub prize_mint: Account<'info, Mint>, // e.g., USDC mint
    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(
        init, 
        payer = buyer, 
        space = 8 + TicketAccount::SIZE, 
        seeds = [TICKET_SEED, lottery.key().as_ref(), &lottery.total_tickets.to_le_bytes()], 
        bump
    )]
    pub ticket_account: Account<'info, TicketAccount>,
    #[account(mut, has_one = prize_mint)] // Ensure correct lottery prize mint
    pub lottery: Account<'info, Lottery>,
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)] // Buyer's token account paying for the ticket
    pub buyer_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [LOTTERY_TOKEN_SEED, lottery.key().as_ref()], // Verify treasury PDA derivation
        bump = lottery.treasury_bump,
        token::mint = prize_mint, // Ensure treasury is for the correct mint
    )]
    pub lottery_treasury: Account<'info, TokenAccount>,
    pub prize_mint: Account<'info, Mint>, // Verify the mint being transferred
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

// // Placeholder Context for RequestRandomness
// #[derive(Accounts)]
// pub struct RequestRandomness<'info> {
//     // ... (all commented out accounts)
// }

// // Placeholder Context for FulfillRandomness
// #[derive(Accounts)]
// pub struct FulfillRandomness<'info> {
//     // ... (all commented out accounts)
// }

// Context for SelectWinner
#[derive(Accounts)]
pub struct SelectWinner<'info> {
    #[account(mut, has_one = admin)] // Ensure only admin can select winner in this test setup
    pub lottery: Account<'info, Lottery>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    #[account(mut, has_one = prize_mint)] // Ensure lottery uses the correct prize mint
    pub lottery: Account<'info, Lottery>,
    #[account(
        mut, 
        constraint = ticket_account.owner == *claimant.key @ ErrorCode::Unauthorized,
        constraint = ticket_account.ticket_id as u64 == lottery.winner_index @ ErrorCode::NotWinner
    )]
    pub ticket_account: Account<'info, TicketAccount>,
    #[account(mut)]
    pub claimant: Signer<'info>,
    #[account(
        mut,
        seeds = [LOTTERY_TOKEN_SEED, lottery.key().as_ref()], // Verify treasury PDA derivation
        bump = lottery.treasury_bump,
        token::mint = prize_mint, // Ensure treasury is for the correct mint
    )]
    pub lottery_treasury: Account<'info, TokenAccount>,
    #[account(
        mut,
        token::mint = prize_mint // Ensure winner account is for the correct mint
    )]
    pub winner_token_account: Account<'info, TokenAccount>,
    pub prize_mint: Account<'info, Mint>, // Pass prize mint for verification
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct Lottery {
    pub admin: Pubkey,
    pub total_tickets: u32,
    pub winner_index: u64,
    pub is_winner_selected: bool,
    pub prize_mint: Pubkey, // Store the mint of the prize token
    pub treasury_bump: u8, // Bump seed for the treasury PDA
    // pub is_prize_claimed: bool, // Optional flag
}

impl Lottery {
    // Add space for new fields: prize_mint (32), treasury_bump (1)
    pub const SIZE: usize = 32 + 4 + 8 + 1 + 32 + 1; 
}

#[account]
pub struct TicketAccount {
    pub owner: Pubkey,
    pub ticket_id: u32,
    // pub is_claimed: bool, // Optional flag
}

impl TicketAccount {
    // Add space for is_claimed (1)
    pub const SIZE: usize = 32 + 4; // + 1;
}

#[error_code]
pub enum ErrorCode {
    #[msg("No tickets have been sold")]
    NoTickets,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Unauthorized access for prize claim")]
    Unauthorized,
    #[msg("The ticket is not the winning ticket")]
    NotWinner,
    #[msg("Winner has not been selected yet")]
    WinnerNotSelected,
    #[msg("Failed to get PDA bump seed")]
    BumpError,
    // Comment out VRF specific errors if not needed for now
    // #[msg("VRF verification failed")]
    // VrfVerificationFailed,
    // #[msg("Invalid VRF oracle fulfilling the request")]
    // InvalidOracle,
    // #[msg("VRF result byte array too short")]
    // VrfResultTooShort,
    // Add more error codes as needed
} 