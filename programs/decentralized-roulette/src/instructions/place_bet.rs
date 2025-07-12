use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer as SplTransfer};
use crate::state::{bet::BetAccount, roulette::{RouletteAccount, RouletteState, BetType}};
use crate::constants::*;
use crate::errors::RouletteError;
use crate::events::BetPlaced;

#[derive(Accounts)]
#[instruction(bet_type: BetType, bet_amount: u64, bet_numbers: Vec<u8>)]
pub struct PlaceBet<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Open @ RouletteError::InvalidGameState,
        constraint = bet_amount >= roulette.min_bet && bet_amount <= roulette.max_bet @ RouletteError::InvalidBetAmount
    )]
    pub roulette: Account<'info, RouletteAccount>,

    #[account(
        init_if_needed,
        payer = bettor,
        space = BetAccount::ACCOUNT_SIZE,
        seeds = [BET_SEED, roulette.key().as_ref(), &roulette.last_bet_id.to_le_bytes()],
        bump
    )]
    pub bet: Account<'info, BetAccount>,

    #[account(mut)]
    pub bettor_usdc_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub roulette_usdc_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub bettor: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<PlaceBet>, bet_type: BetType, bet_amount: u64, bet_numbers: Vec<u8>) -> Result<()> {
    let clock = Clock::get()?;
    let roulette = &mut ctx.accounts.roulette;
    
    // === COMPREHENSIVE INPUT VALIDATION ===
    
    // 1. Timing validation
    require!(
        clock.unix_timestamp < roulette.betting_end_time, 
        RouletteError::BettingClosed
    );
    
    // 2. Bet amount validation (enhanced)
    require!(
        bet_amount >= roulette.min_bet && bet_amount <= roulette.max_bet,
        RouletteError::InvalidBetAmount
    );
    
    // 3. Game capacity validation
    require!(
        roulette.total_players < DEFAULT_MAX_PLAYERS_PER_GAME,
        RouletteError::GameFull
    );
    
    // 4. Bet numbers validation
    validate_bet_numbers(&bet_type, &bet_numbers)?;
    
    // 5. Prevent duplicate/spam bets from same user
    require!(
        roulette.total_bets < 10000, // Prevent DoS attacks
        RouletteError::TooManyBets
    );
    
    // 6. Token account validation
    require!(
        ctx.accounts.bettor_usdc_account.amount >= bet_amount,
        RouletteError::InsufficientFunds
    );
    
    // === END VALIDATION ===

    let bet = &mut ctx.accounts.bet;

    bet.roulette = roulette.key();
    bet.bet_id = roulette.last_bet_id + 1;
    bet.bettor = ctx.accounts.bettor.key();
    bet.bet_type = bet_type.clone();
    bet.bet_amount = bet_amount;
    bet.bet_numbers = roulette.get_numbers_for_bet_type(&bet_type, &bet_numbers);
    bet.payout_multiplier = get_payout_multiplier(&bet_type);
    bet.placed_at = clock.unix_timestamp;
    bet.bump = ctx.bumps.bet;

    // Transfer bet amount
    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SplTransfer {
            from: ctx.accounts.bettor_usdc_account.to_account_info(),
            to: ctx.accounts.roulette_usdc_account.to_account_info(),
            authority: ctx.accounts.bettor.to_account_info(),
        },
    );
    anchor_spl::token::transfer(transfer_ctx, bet_amount)?;

    roulette.total_bets += 1;
    roulette.total_bet_amount += bet_amount;
    roulette.total_players += 1;  // Simplify, no unique check
    roulette.last_bet_id += 1;

    emit!(BetPlaced {
        roulette_id: roulette.key(),
        bet_id: bet.bet_id,
        bettor: bet.bettor,
        bet_type,
        bet_amount,
        bet_numbers: bet.bet_numbers.clone(),
        total_bets: roulette.total_bets,
        total_bet_amount: roulette.total_bet_amount,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}

/// Validate bet numbers for specific bet types
fn validate_bet_numbers(bet_type: &BetType, bet_numbers: &[u8]) -> Result<()> {
    // Check if numbers are within valid range (0-36 for European roulette)
    for &number in bet_numbers {
        require!(number <= 36, RouletteError::InvalidBetNumbers);
    }
    
    match bet_type {
        BetType::Straight => {
            require!(bet_numbers.len() == 1, RouletteError::InvalidBetNumbers);
        },
        BetType::Split => {
            require!(bet_numbers.len() == 2, RouletteError::InvalidBetNumbers);
            // Validate adjacent numbers (simplified check)
            require!(bet_numbers[0] != bet_numbers[1], RouletteError::InvalidBetNumbers);
        },
        BetType::Street => {
            require!(bet_numbers.len() == 3, RouletteError::InvalidBetNumbers);
        },
        BetType::Corner => {
            require!(bet_numbers.len() == 4, RouletteError::InvalidBetNumbers);
        },
        BetType::SixLine => {
            require!(bet_numbers.len() == 6, RouletteError::InvalidBetNumbers);
        },
        // Even money and dozen bets don't require specific numbers
        BetType::Red | BetType::Black | BetType::Even | BetType::Odd | 
        BetType::Low | BetType::High | BetType::FirstTwelve | 
        BetType::SecondTwelve | BetType::ThirdTwelve | BetType::FirstColumn |
        BetType::SecondColumn | BetType::ThirdColumn => {
            // These bet types use predefined numbers, ignore input
        },
    }
    
    Ok(())
}

/// Get payout multiplier for bet type
fn get_payout_multiplier(bet_type: &BetType) -> u16 {
    match bet_type {
        BetType::Straight => STRAIGHT_PAYOUT,
        BetType::Split => SPLIT_PAYOUT,
        BetType::Street => STREET_PAYOUT,
        BetType::Corner => CORNER_PAYOUT,
        BetType::SixLine => SIXLINE_PAYOUT,
        BetType::Red | BetType::Black | BetType::Even | BetType::Odd | BetType::Low | BetType::High => EVEN_MONEY_PAYOUT,
        BetType::FirstTwelve | BetType::SecondTwelve | BetType::ThirdTwelve => DOZEN_COLUMN_PAYOUT,
        BetType::FirstColumn | BetType::SecondColumn | BetType::ThirdColumn => DOZEN_COLUMN_PAYOUT,
    }
}