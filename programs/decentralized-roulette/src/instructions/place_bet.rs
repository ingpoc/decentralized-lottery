use anchor_lang::prelude::*;
use crate::state::{
    global_config::GlobalConfig,
    roulette::{RouletteAccount, RouletteState, BetType},
    bet::BetAccount
};
use crate::constants::*;
use crate::events::BetPlaced;
use crate::errors::RouletteError;

#[derive(Accounts)]
#[instruction(bet_type: BetType, bet_amount: u64, bet_numbers: Vec<u8>)]
pub struct PlaceBet<'info> {
    #[account(
        mut,
        constraint = roulette.state == RouletteState::Open @ RouletteError::InvalidGameState,
        constraint = roulette.total_players < 1000 @ RouletteError::MaxPlayersReached
    )]
    pub roulette: Account<'info, RouletteAccount>,
    
    #[account(
        init,
        payer = bettor,
        space = BetAccount::ACCOUNT_SIZE,
        seeds = [BET_SEED, roulette.key().as_ref(), bettor.key().as_ref(), &roulette.total_bets.to_le_bytes()],
        bump
    )]
    pub bet: Account<'info, BetAccount>,
    
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global_config.bump,
        constraint = !global_config.is_paused @ RouletteError::GamePaused
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub bettor: Signer<'info>,
    
    /// CHECK: USDC mint account
    #[account(
        constraint = usdc_mint.key() == global_config.usdc_mint @ RouletteError::InvalidTokenAccount
    )]
    pub usdc_mint: AccountInfo<'info>,
    
    /// CHECK: Bettor's USDC token account
    #[account(mut)]
    pub bettor_token_account: AccountInfo<'info>,
    
    /// CHECK: Roulette token account (receives bet)
    #[account(mut)]
    pub roulette_token_account: AccountInfo<'info>,
    
    /// CHECK: Token program
    pub token_program: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<PlaceBet>,
    bet_type: BetType,
    bet_amount: u64,
    bet_numbers: Vec<u8>,
) -> Result<()> {
    let roulette = &mut ctx.accounts.roulette;
    let bet = &mut ctx.accounts.bet;
    let clock = Clock::get()?;
    
    // Validate timing
    require!(
        clock.unix_timestamp <= roulette.betting_end_time,
        RouletteError::BettingPeriodEnded
    );
    
    // Validate bet amount
    require!(
        bet_amount >= roulette.min_bet,
        RouletteError::BetBelowMinimum
    );
    require!(
        bet_amount <= roulette.max_bet,
        RouletteError::BetExceedsMaximum
    );
    
    // Validate bet numbers
    validate_bet_numbers(&bet_type, &bet_numbers, &roulette.roulette_type)?;
    
    // TODO: Add token transfer logic using CPI
    // This is commented out for IDL generation
    // transfer(transfer_ctx, bet_amount)?;
    
    // Initialize bet account
    bet.bet_id = roulette.total_bets;
    bet.roulette = roulette.key();
    bet.bettor = ctx.accounts.bettor.key();
    bet.bet_type = bet_type.clone();
    bet.bet_amount = bet_amount;
    bet.bet_numbers = bet_numbers.clone();
    bet.payout_multiplier = get_payout_multiplier(&bet_type);
    bet.is_winner = false;
    bet.payout_amount = 0;
    bet.is_claimed = false;
    bet.placed_at = clock.unix_timestamp;
    bet.claimed_at = None;
    bet.bump = ctx.bumps.bet;
    
    // Update roulette stats
    roulette.total_bets += 1;
    roulette.total_bet_amount += bet_amount;
    
    // Check if this is a new player
    let is_new_player = roulette.total_bets == 1 || 
        ctx.accounts.bettor.key() != roulette.created_by; // Simplified check
    
    if is_new_player {
        roulette.total_players += 1;
    }
    
    // roulette.updated_at = clock.unix_timestamp;
    
    // Emit bet placed event
    emit!(BetPlaced {
        roulette_id: roulette.key(),
        bet_id: bet.bet_id,
        bettor: ctx.accounts.bettor.key(),
        bet_type: bet_type.clone(),
        bet_amount,
        bet_numbers,
        total_bets: roulette.total_bets,
        total_bet_amount: roulette.total_bet_amount,
        timestamp: clock.unix_timestamp,
    });
    
    msg!("Bet placed: {} USDC on {:?}", bet_amount, bet_type);
    
    Ok(())
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

fn validate_bet_numbers(
    bet_type: &BetType,
    bet_numbers: &[u8],
    roulette_type: &crate::state::roulette::RouletteType,
) -> Result<()> {
    let max_number = match roulette_type {
        crate::state::roulette::RouletteType::European => EUROPEAN_ROULETTE_NUMBERS - 1,
        crate::state::roulette::RouletteType::American => AMERICAN_ROULETTE_NUMBERS - 1,
    };
    
    // Validate all numbers are within range
    for &number in bet_numbers {
        require!(number <= max_number, RouletteError::InvalidBetNumbers);
    }
    
    // Validate bet type matches number count
    match bet_type {
        BetType::Straight => {
            require!(bet_numbers.len() == 1, RouletteError::InvalidBetNumbers);
        },
        BetType::Split => {
            require!(bet_numbers.len() == 2, RouletteError::InvalidBetNumbers);
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
        BetType::Red => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::Black => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::Even => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::Odd => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::Low => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::High => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::FirstTwelve => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::SecondTwelve => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::ThirdTwelve => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::FirstColumn => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::SecondColumn => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
        BetType::ThirdColumn => {
            require!(bet_numbers.is_empty(), RouletteError::InvalidBetNumbers);
        },
    }
    
    Ok(())
}