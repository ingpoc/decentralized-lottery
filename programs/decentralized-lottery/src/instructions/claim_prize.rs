use anchor_lang::prelude::*;
use anchor_spl::token::{self, Transfer, Token, TokenAccount};
use crate::state::lottery::{LotteryAccount, LotteryState};
use crate::state::ticket::TicketAccount;
use crate::state::GlobalConfig;
use crate::errors::LotteryError;
use crate::events::PrizeClaimed;

#[derive(Accounts)]
pub struct ClaimPrize<'info> {
    /// CHECK: Lottery account is validated in instruction logic
    #[account(mut)]
    pub lottery_account: AccountInfo<'info>,

    /// CHECK: Ticket account is validated in instruction logic
    #[account(mut)]
    pub ticket_account: AccountInfo<'info>,

    /// CHECK: Global config account is validated in instruction logic
    pub global_config: AccountInfo<'info>,

    #[account(mut)]
    pub winner: Signer<'info>,

    /// CHECK: Lottery token account is validated in instruction logic
    #[account(mut)]
    pub lottery_token_account: AccountInfo<'info>,

    /// CHECK: Winner token account is validated in instruction logic
    #[account(mut)]
    pub winner_token_account: AccountInfo<'info>,

    /// CHECK: Treasury token account is validated in instruction logic
    #[account(mut)]
    pub treasury_token_account: AccountInfo<'info>,

    pub token_program: Program<'info, Token>,
}

pub fn claim_prize_handler(ctx: Context<ClaimPrize>) -> Result<()> {
    let clock = Clock::get()?;
    
    // Deserialize accounts efficiently
    let lottery_data = ctx.accounts.lottery_account.try_borrow_data()?;
    let mut lottery_account: LotteryAccount = LotteryAccount::try_deserialize(&mut lottery_data.as_ref())?;
    drop(lottery_data);
    
    let ticket_data = ctx.accounts.ticket_account.try_borrow_data()?;
    let mut ticket_account: TicketAccount = TicketAccount::try_deserialize(&mut ticket_data.as_ref())?;
    drop(ticket_data);
    
    let config_data = ctx.accounts.global_config.try_borrow_data()?;
    let global_config: GlobalConfig = GlobalConfig::try_deserialize(&mut config_data.as_ref())?;
    drop(config_data);

    // Validate PDAs
    let lottery_seeds = &[
        b"lottery",
        lottery_account.authority.as_ref(),
        &lottery_account.nonce.to_le_bytes()
    ];
    let (expected_lottery_key, _) = Pubkey::find_program_address(lottery_seeds, ctx.program_id);
    require!(expected_lottery_key == ctx.accounts.lottery_account.key(), LotteryError::InvalidAccount);
    
    let config_seeds: &[&[u8]] = &[b"global_config_v2"];
    let (expected_config_key, _) = Pubkey::find_program_address(config_seeds, ctx.program_id);
    require!(expected_config_key == ctx.accounts.global_config.key(), LotteryError::InvalidAccount);

    let lottery_key = ctx.accounts.lottery_account.key();

    // Validate that lottery_token_account is an ATA owned by the lottery PDA.
    // The lottery PDA is the authority that signs CPI transfers out of this vault.
    let lottery_token_data = ctx.accounts.lottery_token_account.try_borrow_data()?;
    let lottery_token: TokenAccount = TokenAccount::try_deserialize(&mut lottery_token_data.as_ref())?;

    require!(
        lottery_token.owner == lottery_key,
        LotteryError::InvalidAccount
    );
    require!(lottery_token.mint == global_config.usdc_mint, LotteryError::InvalidTokenAccount);
    require!(lottery_token.amount >= lottery_account.prize_pool, LotteryError::InsufficientFunds);
    drop(lottery_token_data);

    // Validate lottery state and winning ticket
    require!(lottery_account.state == LotteryState::Completed, LotteryError::InvalidLotteryState);

    let winning_ticket_key = lottery_account.winning_ticket.ok_or(LotteryError::NoWinnerSelected)?;
    require!(winning_ticket_key == ctx.accounts.ticket_account.key(), LotteryError::InvalidWinningTicket);

    // Validate claim status and ownership
    require!(
        !lottery_account.get_is_claimed() &&
        !ticket_account.is_claimed &&
        ticket_account.lottery == ctx.accounts.lottery_account.key() &&
        ticket_account.buyer == ctx.accounts.winner.key(),
        LotteryError::InvalidClaim
    );

    // Calculate treasury fee and winner payout
    let prize_pool = lottery_account.prize_pool;
    let treasury_fee = prize_pool
        .checked_mul(global_config.treasury_fee_percentage as u64)
        .ok_or(LotteryError::ArithmeticOverflow)?
        .checked_div(10000)
        .ok_or(LotteryError::ArithmeticOverflow)?;

    let winner_payout = prize_pool.checked_sub(treasury_fee).ok_or(LotteryError::ArithmeticOverflow)?;

    // Validate token accounts
    let winner_token_data = ctx.accounts.winner_token_account.try_borrow_data()?;
    let winner_token: TokenAccount = TokenAccount::try_deserialize(&mut winner_token_data.as_ref())?;
    require!(
        winner_token.mint == global_config.usdc_mint && 
        winner_token.owner == ctx.accounts.winner.key(),
        LotteryError::InvalidTokenAccount
    );
    drop(winner_token_data);

    let treasury_token_data = ctx.accounts.treasury_token_account.try_borrow_data()?;
    let treasury_token: TokenAccount = TokenAccount::try_deserialize(&mut treasury_token_data.as_ref())?;
    require!(
        treasury_token.mint == global_config.usdc_mint &&
        ctx.accounts.treasury_token_account.key() == global_config.treasury_token_account,
        LotteryError::InvalidTokenAccount
    );
    drop(treasury_token_data);

    // CPI signer seeds — the lottery PDA is the owner of the vault ATA.
    // We sign with the lottery PDA's derivation seeds so the SPL token program
    // accepts it as the transfer authority.
    let (lottery_key_addr, lottery_bump) = Pubkey::find_program_address(
        &[b"lottery", lottery_account.authority.as_ref(), &lottery_account.nonce.to_le_bytes()],
        ctx.program_id,
    );
    require!(lottery_key_addr == lottery_key, LotteryError::InvalidAccount);

    let authority_seeds = &[
        b"lottery".as_slice(),
        lottery_account.authority.as_ref(),
        &lottery_account.nonce.to_le_bytes(),
        &[lottery_bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[&authority_seeds[..]];

    // Transfer treasury fee — authority is the lottery PDA (owner of the vault ATA)
    if treasury_fee > 0 {
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.lottery_token_account.clone(),
                to: ctx.accounts.treasury_token_account.clone(),
                authority: ctx.accounts.lottery_account.clone(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, treasury_fee)?;
    }

    // Transfer winner payout
    if winner_payout > 0 {
        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.lottery_token_account.clone(),
                to: ctx.accounts.winner_token_account.clone(),
                authority: ctx.accounts.lottery_account.clone(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, winner_payout)?;
    }

    // Update state
    lottery_account.set_is_claimed(true);
    ticket_account.is_claimed = true;

    // Write updated data back
    let mut lottery_data_mut = ctx.accounts.lottery_account.try_borrow_mut_data()?;
    lottery_account.try_serialize(&mut lottery_data_mut.as_mut())?;
    drop(lottery_data_mut);

    let mut ticket_data_mut = ctx.accounts.ticket_account.try_borrow_mut_data()?;
    ticket_account.try_serialize(&mut ticket_data_mut.as_mut())?;
    drop(ticket_data_mut);

    emit!(PrizeClaimed {
        lottery_id: ctx.accounts.lottery_account.key(),
        ticket_id: ticket_account.id,
        winner: ticket_account.buyer,
        prize_pool: lottery_account.prize_pool,
        treasury_fee,
        winner_payout,
        timestamp: clock.unix_timestamp,
    });

    Ok(())
}
