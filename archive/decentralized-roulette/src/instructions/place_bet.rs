use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount, Transfer as SplTransfer};
use crate::state::{bet::BetAccount, roulette::{RouletteAccount, RouletteState, BetType}};
use crate::constants::*;

fn calculate_max_payout(bet_type: &BetType, bet_amount: u64) -> Result<u64> {
    let max_multiplier = match bet_type {
        BetType::Straight => 35,        // 35:1 - highest payout
        BetType::Split => 17,           // 17:1
        BetType::Street => 11,          // 11:1
        BetType::Corner => 8,           // 8:1
        BetType::SixLine => 5,          // 5:1
        BetType::Red | BetType::Black | 
        BetType::Even | BetType::Odd | 
        BetType::Low | BetType::High => 1, // 1:1
        BetType::FirstTwelve | BetType::SecondTwelve | 
        BetType::ThirdTwelve | BetType::FirstColumn |
        BetType::SecondColumn | BetType::ThirdColumn => 2, // 2:1
    };
    
    bet_amount
        .checked_mul(max_multiplier + 1)
        .ok_or(RouletteError::PayoutOverflow.into())
}
use crate::errors::RouletteError;
use crate::events::BetPlaced;

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    /// CHECK: Roulette account is validated in instruction logic
    #[account(mut)]
    pub roulette: AccountInfo<'info>,

    /// CHECK: Global config account is validated in instruction logic
    pub global_config: AccountInfo<'info>,

    /// CHECK: Bet account is validated in instruction logic
    #[account(
        init_if_needed,
        payer = bettor,
        space = BetAccount::ACCOUNT_SIZE
    )]
    pub bet: AccountInfo<'info>,

    /// CHECK: Bettor USDC account is validated in instruction logic
    #[account(mut)]
    pub bettor_usdc_account: AccountInfo<'info>,

    /// CHECK: Roulette USDC account is validated in instruction logic
    #[account(mut)]
    pub roulette_usdc_account: AccountInfo<'info>,

    #[account(mut)]
    pub bettor: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

fn validate_bet_context(
    ctx: &Context<PlaceBet>,
    bet_type: &BetType,
    bet_amount: u64,
    bet_numbers: &[u8],
) -> Result<(RouletteAccount, i64)> {
    let clock = Clock::get()?;
    
    let roulette_data = ctx.accounts.roulette.try_borrow_data()?;
    let roulette: RouletteAccount = RouletteAccount::try_deserialize(&mut roulette_data.as_ref())?;
    drop(roulette_data);
    
    let config_seeds = &[GLOBAL_CONFIG_SEED];
    let (expected_config_key, _) = Pubkey::find_program_address(config_seeds, ctx.program_id);
    require!(expected_config_key == ctx.accounts.global_config.key(), RouletteError::InvalidAccount);

    // Enhanced input validation with security checks
    require!(roulette.state == RouletteState::Open, RouletteError::InvalidGameState);
    require!(clock.unix_timestamp < roulette.betting_end_time, RouletteError::BettingClosed);
    
    // Validate bet amount bounds with overflow protection
    require!(bet_amount > 0, RouletteError::InvalidBetAmount);
    require!(bet_amount >= roulette.min_bet, RouletteError::BetBelowMinimum);
    require!(bet_amount <= roulette.max_bet, RouletteError::BetExceedsMaximum);
    
    // Check capacity limits
    require!(roulette.total_players < DEFAULT_MAX_PLAYERS_PER_GAME, RouletteError::GameFull);
    require!(roulette.total_bets < 10000, RouletteError::TooManyBets);
    
    // Validate bet type and numbers
    validate_bet_numbers(bet_type, bet_numbers)?;
    
    // Check for arithmetic overflow in total bet amount
    let new_total = roulette.total_bet_amount
        .checked_add(bet_amount)
        .ok_or(RouletteError::ArithmeticOverflow)?;
    
    // Validate treasury can handle potential maximum payout
    let max_possible_payout = calculate_max_payout(bet_type, bet_amount)?;
    require!(
        new_total >= max_possible_payout,
        RouletteError::InsufficientTreasury
    );
    
    Ok((roulette, clock.unix_timestamp))
}

fn validate_token_accounts(ctx: &Context<PlaceBet>, bet_amount: u64, bet_type: &BetType) -> Result<()> {
    let bettor_token_data = ctx.accounts.bettor_usdc_account.try_borrow_data()?;
    let bettor_token: TokenAccount = TokenAccount::try_deserialize(&mut bettor_token_data.as_ref())?;
    require!(bettor_token.amount >= bet_amount, RouletteError::InsufficientFunds);
    drop(bettor_token_data);
    
    let roulette_token_data = ctx.accounts.roulette_usdc_account.try_borrow_data()?;
    let roulette_token: TokenAccount = TokenAccount::try_deserialize(&mut roulette_token_data.as_ref())?;
    
    let max_payout = bet_amount.checked_mul(get_payout_multiplier(bet_type) as u64)
        .ok_or(RouletteError::ArithmeticOverflow)?;
    
    require!(roulette_token.amount >= max_payout, RouletteError::InsufficientTreasury);
    let treasury_ratio = roulette_token.amount.checked_div(100).unwrap_or(0);
    require!(max_payout <= treasury_ratio, RouletteError::BetTooLarge);
    drop(roulette_token_data);
    
    Ok(())
}

fn process_bet_placement(
    ctx: &Context<PlaceBet>,
    mut roulette: RouletteAccount,
    bet_type: &BetType,
    bet_amount: u64,
    bet_numbers: &[u8],
    timestamp: i64,
) -> Result<()> {
    let bet_id = roulette.last_bet_id + 1;
    let roulette_key = ctx.accounts.roulette.key();
    let bet_id_bytes = bet_id.to_le_bytes();
    let bet_seeds = &[BET_SEED, roulette_key.as_ref(), &bet_id_bytes];
    let (expected_bet_key, bet_bump) = Pubkey::find_program_address(bet_seeds, ctx.program_id);
    require!(expected_bet_key == ctx.accounts.bet.key(), RouletteError::InvalidAccount);

    let bet_account = BetAccount {
        roulette: ctx.accounts.roulette.key(),
        bet_id,
        bettor: ctx.accounts.bettor.key(),
        bet_type: bet_type.clone(),
        bet_amount,
        bet_numbers: roulette.get_numbers_for_bet_type(bet_type, bet_numbers),
        payout_multiplier: get_payout_multiplier(bet_type),
        placed_at: timestamp,
        is_winner: false,
        payout_amount: 0,
        is_claimed: false,
        claimed_at: None,
        bump: bet_bump,
    };

    let mut bet_data = ctx.accounts.bet.try_borrow_mut_data()?;
    bet_account.try_serialize(&mut bet_data.as_mut())?;
    drop(bet_data);

    let transfer_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        SplTransfer {
            from: ctx.accounts.bettor_usdc_account.clone(),
            to: ctx.accounts.roulette_usdc_account.clone(),
            authority: ctx.accounts.bettor.to_account_info(),
        },
    );
    anchor_spl::token::transfer(transfer_ctx, bet_amount)?;

    roulette.total_bets += 1;
    roulette.total_bet_amount += bet_amount;
    roulette.total_players += 1;
    roulette.last_bet_id = bet_id;

    let mut roulette_data_mut = ctx.accounts.roulette.try_borrow_mut_data()?;
    roulette.try_serialize(&mut roulette_data_mut.as_mut())?;
    drop(roulette_data_mut);

    emit!(BetPlaced {
        roulette_id: ctx.accounts.roulette.key(),
        bet_id,
        bettor: ctx.accounts.bettor.key(),
        bet_type: bet_type.clone(),
        bet_amount,
        bet_numbers: bet_account.bet_numbers.clone(),
        total_bets: roulette.total_bets,
        total_bet_amount: roulette.total_bet_amount,
        timestamp,
    });

    Ok(())
}

pub fn handler(ctx: Context<PlaceBet>, bet_type: BetType, bet_amount: u64, bet_numbers: Vec<u8>) -> Result<()> {
    let (roulette, timestamp) = validate_bet_context(&ctx, &bet_type, bet_amount, &bet_numbers)?;
    validate_token_accounts(&ctx, bet_amount, &bet_type)?;
    process_bet_placement(&ctx, roulette, &bet_type, bet_amount, &bet_numbers, timestamp)?;
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
