//src/errors.rs
use anchor_lang::error_code;

#[error_code]
pub enum LotteryError {
    #[msg("Lottery type not supported")]
    UnsupportedLotteryType,
    #[msg("Invalid ticket price")]
    InvalidTicketPrice,
    #[msg("Invalid prize pool")]
    InvalidPrizePool,
    #[msg("Lottery draw time invalid")]
    InvalidDrawTime,
    #[msg("Ticket purchase amount invalid")]
    InvalidTicketAmount,
    #[msg("Ticket purchase limit reached")]
    TicketPurchaseLimitReached,
    #[msg("Lottery is not open")]
    LotteryNotOpen,
    #[msg("Lottery is drawing")]
    LotteryDrawing,
    #[msg("Lottery is completed")]
    LotteryCompleted,
    #[msg("Lottery is expired")]
    LotteryExpired,
    #[msg("Invalid lottery state")]
    InvalidLotteryState,
    #[msg("Invalid account owner")]
    InvalidAccountOwner,
    #[msg("Invalid instruction input")]
    InvalidInstructionInput,
    #[msg("Safe Math Error")]
    SafeMathError,
    #[msg("Prize claim time expired")]
    PrizeClaimTimeExpired,
    #[msg("Invalid prize tier")]
    InvalidPrizeTier,
    #[msg("Treasury withdrawal time lock not yet reached")]
    TreasuryWithdrawalTimeLockNotReached,
    #[msg("Invalid treasury multisig")]
    InvalidTreasuryMultisig,
    #[msg("Token transfer failed")]
    TokenTransferFailed,
    #[msg("Invalid token account")]
    InvalidTokenAccount,
    #[msg("Oracle price feed error")]
    OraclePriceFeedError,
    #[msg("Randomness generation failed")]
    RandomnessGenerationFailed,
    #[msg("Unauthorized access")]
    UnauthorizedAccess,
    #[msg("Invalid state transition")]
    InvalidStateTransition,
    #[msg("Lottery cannot be cancelled in current state")]
    InvalidCancellation,
    #[msg("Only admin can perform this action")]
    AdminRequired,
    #[msg("Lottery is cancelled")]
    LotteryCancelled,
}