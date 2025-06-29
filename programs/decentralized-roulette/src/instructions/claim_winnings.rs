use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState, BetType},
    bet::BetAccount
};
use crate::constants::*;
use crate::events::WinningsClaimed;
use crate::errors::RouletteError;

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Completed @ RouletteError::InvalidGameState,
        constraint = roulette.randomness_fulfilled @ RouletteError::RandomnessNotFulfilled,
        constraint = roulette.winning_number.is_some() @ RouletteError::RandomnessNotFulfilled
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        mut,
        constraint = bet.roulette == roulette.key() @ RouletteError::InvalidBetNumbers,
        constraint = bet.bettor == claimer.key() @ RouletteError::InvalidAuthority,
        constraint = !bet.is_claimed @ RouletteError::WinningsAlreadyClaimed
    )]
    pub bet: Account<'info, BetAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub claimer: Signer<'info>,
    
    /// CHECK: Claimer's USDC token account
    #[account(mut)]
    pub claimer_token_account: AccountInfo<'info>,
    
    /// CHECK: Roulette token account (pays out winnings)
    #[account(mut)]
    pub roulette_token_account: AccountInfo<'info>,
    
    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
}

pub fn handler(ctx: Context<ClaimWinnings>) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let bet = &mut ctx.accounts.bet;
    let clock = Clock::get()?;
    
    let winning_number = roulette.winning_number.unwrap();
    
    // Check if this bet is a winner
    let is_winning = is_winning_bet(&bet.bet_type, &bet.bet_numbers, winning_number);
    require!(is_winning, RouletteError::NotAWinningBet);
    
    // Calculate payout
    let payout_multiplier = get_payout_multiplier(&bet.bet_type);
    let payout_amount = bet.bet_amount * payout_multiplier as u64;
    
    // Update bet account
    bet.is_winner = true;
    bet.payout_amount = payout_amount;
    bet.is_claimed = true;
    bet.claimed_at = Some(clock.unix_timestamp);
    
    // Update roulette stats
    roulette.total_payouts += payout_amount;
    // roulette.updated_at = clock.unix_timestamp;
    
    // TODO: Add token transfer logic using CPI
    // This is commented out for IDL generation
    // transfer(transfer_ctx, payout_amount)?;
    
    // Emit winnings claimed event
    emit!(WinningsClaimed {
        roulette_id: roulette.key(),
        bet_id: bet.bet_id,
        winner: ctx.accounts.claimer.key(),
        payout_amount,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Winnings claimed: {} USDC for bet {}", payout_amount, bet.bet_id);
    
    Ok(())
}

fn is_winning_bet(bet_type: &BetType, bet_numbers: &[u8], winning_number: u8) -> bool {
    match bet_type {
        BetType::Straight => bet_numbers.contains(&winning_number),
        BetType::Split => bet_numbers.contains(&winning_number),
        BetType::Street => bet_numbers.contains(&winning_number),
        BetType::Corner => bet_numbers.contains(&winning_number),
        BetType::SixLine => bet_numbers.contains(&winning_number),
        BetType::Red => {
            if winning_number == 0 { return false; }
            RED_NUMBERS.contains(&winning_number)
        },
        BetType::Black => {
            if winning_number == 0 { return false; }
            BLACK_NUMBERS.contains(&winning_number)
        },
        BetType::Even => {
            if winning_number == 0 { return false; }
            winning_number % 2 == 0
        },
        BetType::Odd => {
            if winning_number == 0 { return false; }
            winning_number % 2 == 1
        },
        BetType::Low => {
            winning_number >= 1 && winning_number <= 18
        },
        BetType::High => {
            winning_number >= 19 && winning_number <= 36
        },
        BetType::FirstTwelve => {
            winning_number >= 1 && winning_number <= 12
        },
        BetType::SecondTwelve => {
            winning_number >= 13 && winning_number <= 24
        },
        BetType::ThirdTwelve => {
            winning_number >= 25 && winning_number <= 36
        },
        BetType::FirstColumn => {
            FIRST_COLUMN.contains(&winning_number)
        },
        BetType::SecondColumn => {
            SECOND_COLUMN.contains(&winning_number)
        },
        BetType::ThirdColumn => {
            THIRD_COLUMN.contains(&winning_number)
        },
    }
}

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