use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};
use anchor_spl::associated_token::AssociatedToken;

declare_id!("9SL8XkX3pvqZ2fjiLMhCFfQn7Gfmpd9ru8rtHFsAPVgq");

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use state::GlobalConfig;
use state::lottery::{LotteryAccount, LotteryType, LotteryState};
use state::ticket::TicketAccount;
use errors::LotteryError;
use events::{LotteryCreated, TicketPurchased, LotteryStateChanged, PrizeClaimed, LotteryWinnerDetermined};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = admin,
        space = GlobalConfig::ACCOUNT_SIZE,
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: This is the USDC mint account
    pub usdc_mint: AccountInfo<'info>,
    /// CHECK: This is the treasury token account
    #[account(mut)]
    pub treasury_token_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(lottery_type_enum: LotteryType)]
pub struct CreateLottery<'info> {
    #[account(
        init,
        payer = creator,
        space = LotteryAccount::ACCOUNT_SIZE,
        seeds = [b"lottery", creator.key().as_ref(), &Clock::get().unwrap().unix_timestamp.to_le_bytes()],
        bump
    )]
    pub lottery_account: Account<'info, LotteryAccount>,
    
    #[account(
        seeds = [b"global_config"],
        bump,
        constraint = global_config.admin == creator.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Account<'info, GlobalConfig>,
    
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BuyTicket<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.created_at.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Open @ LotteryError::LotteryNotOpen
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        init,
        payer = user,
        space = TicketAccount::ACCOUNT_SIZE,
        seeds = [
            b"ticket", 
            lottery_account.key().as_ref(), 
            &(lottery_account.last_ticket_id + 1).to_le_bytes()
        ],
        bump
    )]
    pub ticket_account: Account<'info, TicketAccount>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        constraint = user_token_account.owner == user.key() @ LotteryError::InvalidTokenAccount,
        constraint = user_token_account.mint == global_config.usdc_mint @ LotteryError::InvalidTokenAccount
    )]
    pub user_token_account: Account<'info, TokenAccount>,
    
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = usdc_mint,
        associated_token::authority = lottery_account,
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,
    
    #[account(constraint = usdc_mint.key() == global_config.usdc_mint)]
    pub usdc_mint: Account<'info, Mint>,
    
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransitionState<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.created_at.to_le_bytes()],
        bump
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        seeds = [b"global_config"],
        bump,
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SelectWinner<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.created_at.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Drawing @ LotteryError::InvalidLotteryState
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        seeds = [b"global_config"],
        bump,
        constraint = global_config.admin == admin.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub admin: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(
        mut,
        seeds = [b"global_config"],
        bump,
        constraint = global_config.admin == admin.key() @ LotteryError::AdminRequired
    )]
    pub global_config: Account<'info, GlobalConfig>,
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: This is the new USDC mint account
    pub new_usdc_mint: AccountInfo<'info>,
    /// CHECK: This is the new treasury token account
    pub new_treasury_token_account: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    #[account(
        mut,
        seeds = [b"lottery", lottery_account.authority.as_ref(), &lottery_account.created_at.to_le_bytes()],
        bump,
        constraint = lottery_account.state == LotteryState::Completed @ LotteryError::InvalidLotteryState,
        constraint = lottery_account.winning_ticket.is_some() @ LotteryError::NoWinnerSelected,
        constraint = lottery_account.winning_ticket.unwrap() == ticket_account.key() @ LotteryError::InvalidWinningTicket,
        constraint = !lottery_account.is_claimed @ LotteryError::LotteryAlreadyClaimed
    )]
    pub lottery_account: Account<'info, LotteryAccount>,

    #[account(
        mut,
        constraint = !ticket_account.is_claimed @ LotteryError::TicketAlreadyClaimed,
        constraint = ticket_account.lottery == lottery_account.key() @ LotteryError::TicketNotForThisLottery,
        constraint = ticket_account.buyer == winner.key() @ LotteryError::UnauthorizedAccess
    )]
    pub ticket_account: Account<'info, TicketAccount>,

    #[account(
        seeds = [b"global_config"],
        bump
    )]
    pub global_config: Account<'info, GlobalConfig>,

    #[account(mut)]
    pub winner: Signer<'info>,
    
    #[account(
        mut,
        associated_token::mint = usdc_mint,
        associated_token::authority = lottery_account,
    )]
    pub lottery_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = winner_token_account.owner == winner.key() @ LotteryError::InvalidTokenAccount
    )]
    pub winner_token_account: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = treasury_token_account.key() == global_config.treasury_token_account @ LotteryError::InvalidTokenAccount
    )]
    pub treasury_token_account: Account<'info, TokenAccount>,
    
    #[account(constraint = usdc_mint.key() == global_config.usdc_mint)]
    pub usdc_mint: Account<'info, Mint>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[program]
pub mod decentralized_lottery {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let global_config = &mut ctx.accounts.global_config;
        global_config.admin = ctx.accounts.admin.key();
        global_config.treasury_fee_percentage = 250;
        global_config.usdc_mint = ctx.accounts.usdc_mint.key();
        global_config.treasury_token_account = ctx.accounts.treasury_token_account.key();
        Ok(())
    }

    pub fn create_lottery(
        ctx: Context<CreateLottery>,
        lottery_type_enum: LotteryType,
        ticket_price: u64,
        draw_time: i64,
        target_prize_pool: u64,
    ) -> Result<()> {
        let lottery_account = &mut ctx.accounts.lottery_account;
        let global_config = &ctx.accounts.global_config;
        let clock = Clock::get()?;

        // Validate inputs
        if ticket_price == 0 {
            return Err(LotteryError::InvalidTicketPrice.into());
        }
        
        if draw_time <= clock.unix_timestamp {
            return Err(LotteryError::InvalidDrawTime.into());
        }

        // Initialize Lottery Account
        lottery_account.lottery_type = lottery_type_enum.clone();
        lottery_account.ticket_price = ticket_price;
        lottery_account.draw_time = draw_time;
        lottery_account.prize_pool = 0;
        lottery_account.target_prize_pool = target_prize_pool;
        lottery_account.total_tickets = 0;
        lottery_account.winning_ticket = None;
        lottery_account.state = LotteryState::Created;
        lottery_account.created_by = ctx.accounts.creator.key();
        lottery_account.authority = ctx.accounts.creator.key();
        lottery_account.global_config = global_config.key();
        lottery_account.auto_transition = false;
        lottery_account.last_ticket_id = 0;
        lottery_account.oracle_pubkey = None;
        lottery_account.vrf_client = None;
        lottery_account.vrf_randomness = None;
        lottery_account.vrf_request_account = None;
        lottery_account.is_prize_pool_locked = false;
        lottery_account.is_claimed = false;
        lottery_account.created_at = clock.unix_timestamp;
        lottery_account.completed_at = None;

        emit!(LotteryCreated {
            lottery_id: lottery_account.key(),
            lottery_type: lottery_type_enum.to_string(),
            ticket_price,
            draw_time,
            target_prize_pool,
        });

        Ok(())
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>) -> Result<()> {
        let lottery_account = &mut ctx.accounts.lottery_account;
        let ticket_account = &mut ctx.accounts.ticket_account;
        let user = &ctx.accounts.user;
        let clock = Clock::get()?;
        
        // Additional validation: Check if draw time has passed
        if clock.unix_timestamp >= lottery_account.draw_time && lottery_account.state == LotteryState::Open {
            lottery_account.state = if lottery_account.total_tickets > 0 { LotteryState::Drawing } else { LotteryState::Expired };
            emit!(LotteryStateChanged {
                lottery_id: lottery_account.key(),
                previous_state: LotteryState::Open,
                new_state: lottery_account.state.clone(),
                timestamp: clock.unix_timestamp,
                total_tickets_sold: lottery_account.total_tickets,
                current_prize_pool: lottery_account.prize_pool,
            });
            if lottery_account.state == LotteryState::Expired {
                lottery_account.completed_at = Some(clock.unix_timestamp);
            }
            return Err(LotteryError::LotteryNotOpen.into());
        }
        
        if lottery_account.state != LotteryState::Open {
            return Err(LotteryError::LotteryNotOpen.into());
        }

        let ticket_id = lottery_account.last_ticket_id.checked_add(1).ok_or(LotteryError::ArithmeticOverflow)?;
        lottery_account.last_ticket_id = ticket_id;
        lottery_account.total_tickets = lottery_account.total_tickets.checked_add(1).ok_or(LotteryError::ArithmeticOverflow)?;
        lottery_account.prize_pool = lottery_account.prize_pool.checked_add(lottery_account.ticket_price).ok_or(LotteryError::ArithmeticOverflow)?;

        // Initialize ticket account
        ticket_account.lottery = lottery_account.key();
        ticket_account.buyer = user.key();
        ticket_account.id = ticket_id;
        ticket_account.is_claimed = false;
        
        // Transfer USDC from user to lottery
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.lottery_token_account.to_account_info(),
            authority: user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, lottery_account.ticket_price)?;

        emit!(TicketPurchased {
            lottery_id: lottery_account.key(),
            ticket_id,
            buyer: user.key(),
            number_of_tickets: 1,
            total_cost: lottery_account.ticket_price,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    pub fn transition_state(ctx: Context<TransitionState>, next_state_param: LotteryState) -> Result<()> {
        let lottery_account = &mut ctx.accounts.lottery_account;
        let clock = Clock::get()?;
        let current_state = lottery_account.state.clone();

        if !current_state.can_transition_to(&next_state_param) {
            return Err(LotteryError::InvalidStateTransition.into());
        }

        match (&current_state, &next_state_param) {
            (LotteryState::Created, LotteryState::Open) => {
            },
            (LotteryState::Open, LotteryState::Drawing) => {
                if lottery_account.draw_time > clock.unix_timestamp {
                    return Err(LotteryError::InvalidDrawTime.into());
                }
                if lottery_account.total_tickets == 0 {
                    lottery_account.state = LotteryState::Expired;
                    lottery_account.completed_at = Some(clock.unix_timestamp);
                    emit!(LotteryStateChanged {
                        lottery_id: lottery_account.key(),
                        previous_state: current_state,
                        new_state: LotteryState::Expired,
                        timestamp: clock.unix_timestamp,
                        total_tickets_sold: lottery_account.total_tickets,
                        current_prize_pool: lottery_account.prize_pool,
                    });
                    return Ok(());
                }
            },
            (LotteryState::Drawing, LotteryState::Completed) => {
                if lottery_account.winning_ticket.is_none() {
                     return Err(LotteryError::NoWinnerSelected.into());
                }
                lottery_account.completed_at = Some(clock.unix_timestamp);
            },
            (_, LotteryState::Cancelled) => {
                if !current_state.can_cancel() {
                    return Err(LotteryError::InvalidCancellation.into());
                }
                lottery_account.completed_at = Some(clock.unix_timestamp);
            }
            _ => {
            }
        }

        lottery_account.state = next_state_param.clone();

        emit!(LotteryStateChanged {
            lottery_id: lottery_account.key(),
            previous_state: current_state,
            new_state: next_state_param,
            timestamp: clock.unix_timestamp,
            total_tickets_sold: lottery_account.total_tickets,
            current_prize_pool: lottery_account.prize_pool,
        });
        Ok(())
    }

    pub fn select_winner(ctx: Context<SelectWinner>) -> Result<()> {
        let lottery_account = &mut ctx.accounts.lottery_account;
        let clock = Clock::get()?;

        if lottery_account.state != LotteryState::Drawing {
            return Err(LotteryError::InvalidLotteryState.into());
        }

        if lottery_account.draw_time > clock.unix_timestamp {
            return Err(LotteryError::InvalidDrawTime.into());
        }
        
        if lottery_account.total_tickets == 0 {
            lottery_account.state = LotteryState::Expired;
            lottery_account.completed_at = Some(clock.unix_timestamp);
            emit!(LotteryStateChanged {
                lottery_id: lottery_account.key(),
                previous_state: LotteryState::Drawing,
                new_state: LotteryState::Expired,
                timestamp: clock.unix_timestamp,
                total_tickets_sold: 0,
                current_prize_pool: 0,
            });
            return Ok(());
        }

        // Use simple pseudo-random for winner selection
        let winning_ticket_id = (clock.unix_timestamp as u64 % lottery_account.total_tickets) + 1;
        
        // Derive the winning ticket PDA
        let (winning_ticket_pda, _) = Pubkey::find_program_address(
            &[
                b"ticket",
                lottery_account.key().as_ref(),
                &winning_ticket_id.to_le_bytes(),
            ],
            ctx.program_id,
        );
        
        lottery_account.winning_ticket = Some(winning_ticket_pda);
        lottery_account.is_prize_pool_locked = true;
        let previous_state = lottery_account.state.clone();
        lottery_account.state = LotteryState::Completed;
        lottery_account.completed_at = Some(clock.unix_timestamp);

        emit!(LotteryWinnerDetermined {
            lottery_id: lottery_account.key(),
            previous_state,
            new_state: LotteryState::Completed,
            winner: Pubkey::default(), // Would need to fetch ticket account to get actual winner
            randomness: winning_ticket_id,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
    
    pub fn claim_prize(ctx: Context<ClaimPrize>) -> Result<()> {
        let lottery_account = &mut ctx.accounts.lottery_account;
        let ticket_account = &mut ctx.accounts.ticket_account;
        let global_config = &ctx.accounts.global_config;

        // Calculate treasury fee and winner payout
        let treasury_fee = lottery_account.prize_pool
            .checked_mul(global_config.treasury_fee_percentage as u64)
            .ok_or(LotteryError::ArithmeticOverflow)?
            .checked_div(10000) // basis points
            .ok_or(LotteryError::ArithmeticOverflow)?;
            
        let winner_payout = lottery_account.prize_pool
            .checked_sub(treasury_fee)
            .ok_or(LotteryError::ArithmeticOverflow)?;

        // Transfer treasury fee
        if treasury_fee > 0 {
            let authority_seeds: &[&[&[u8]]] = &[&[
                b"lottery",
                lottery_account.authority.as_ref(),
                &lottery_account.created_at.to_le_bytes(),
                &[ctx.bumps.lottery_account],
            ]];
            
            let cpi_accounts = Transfer {
                from: ctx.accounts.lottery_token_account.to_account_info(),
                to: ctx.accounts.treasury_token_account.to_account_info(),
                authority: lottery_account.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, authority_seeds);
            token::transfer(cpi_ctx, treasury_fee)?;
        }

        // Transfer winner payout
        if winner_payout > 0 {
            let authority_seeds: &[&[&[u8]]] = &[&[
                b"lottery",
                lottery_account.authority.as_ref(),
                &lottery_account.created_at.to_le_bytes(),
                &[ctx.bumps.lottery_account],
            ]];
            
            let cpi_accounts = Transfer {
                from: ctx.accounts.lottery_token_account.to_account_info(),
                to: ctx.accounts.winner_token_account.to_account_info(),
                authority: lottery_account.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, authority_seeds);
            token::transfer(cpi_ctx, winner_payout)?;
        }

        lottery_account.is_claimed = true;
        ticket_account.is_claimed = true;

        msg!("Prize claimed for lottery {} by ticket {}", lottery_account.key(), ticket_account.key());

        emit!(PrizeClaimed {
            lottery_id: lottery_account.key(),
            ticket_id: ticket_account.id,
            winner: ticket_account.buyer,
            prize_pool: lottery_account.prize_pool,
            treasury_fee,
            winner_payout,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }
    
    pub fn update_config(ctx: Context<UpdateConfig>) -> Result<()> {
        let global_config = &mut ctx.accounts.global_config;
        
        global_config.usdc_mint = ctx.accounts.new_usdc_mint.key();
        global_config.treasury_token_account = ctx.accounts.new_treasury_token_account.key();
        
        msg!("Config updated: USDC mint={}, Treasury account={}", 
            global_config.usdc_mint, 
            global_config.treasury_token_account
        );
        
        Ok(())
    }
} 