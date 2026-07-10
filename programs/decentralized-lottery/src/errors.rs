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
    #[msg("Invalid token mint")]
    InvalidTokenMint,
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
    #[msg("The lottery system is currently paused by admin")]
    LotteryPaused,
    #[msg("Lottery is not open for ticket purchases.")]
    LotteryNotOpenForTicketPurchases,
    #[msg("Lottery prize has already been claimed.")]
    LotteryAlreadyClaimed,
    #[msg("Failed to derive PDA.")]
    PDADerivationError,
    #[msg("Provided ticket PDA does not match the winning ticket stored in the lottery.")]
    InvalidWinningTicket,
    #[msg("The provided ticket has already been claimed or refunded.")]
    TicketAlreadyClaimed,
    #[msg("Lottery is not in a state where refunds can be claimed (must be Cancelled or Expired).")]
    InvalidStateForRefund,
    #[msg("The input parameters are invalid.")]
    InvalidInput,
    #[msg("The ticket sale has ended.")]
    TicketSaleEnded,
    #[msg("The lottery has already been drawn.")]
    LotteryAlreadyDrawn,
    #[msg("There are no tickets in this lottery.")]
    NoTickets,
    #[msg("Insufficient tickets sold to proceed with the draw.")]
    InsufficientTicketsSold,
    #[msg("The lottery has not been drawn yet.")]
    LotteryNotDrawn,
    #[msg("The specified ticket is not eligible for refund.")]
    TicketNotEligibleForRefund,
    #[msg("The lottery has not expired yet.")]
    LotteryNotExpired,
    #[msg("VRF account is invalid.")]
    InvalidVrfAccount,
    #[msg("Insufficient funds for this operation.")]
    InsufficientFunds,
    #[msg("Arithmetic overflow error.")]
    ArithmeticOverflow,
    #[msg("Ticket does not belong to this lottery.")]
    TicketNotForThisLottery,
    #[msg("No winner has been selected yet.")]
    NoWinnerSelected,
    #[msg("The prize pool is empty.")]
    EmptyPrizePool,
    #[msg("Insufficient funds in the prize pool.")]
    InsufficientPrizeFunds,
    #[msg("Invalid mint address.")]
    InvalidMint,
    #[msg("Randomness has already been fulfilled.")]
    RandomnessAlreadyFulfilled,
    #[msg("VRF request key is not set.")]
    VrfRequestKeyNotSet,
    #[msg("VRF account mismatch.")]
    VrfAccountMismatch,
    #[msg("Randomness is not fulfilled.")]
    RandomnessNotFulfilled,
    #[msg("Randomness is not available.")]
    RandomnessNotAvailable,
    #[msg("Winner has already been selected.")]
    WinnerAlreadySelected,
    #[msg("No tickets were sold.")]
    NoTicketsSold,
    #[msg("Draw time has not been reached yet.")]
    DrawTimeNotReached,
    #[msg("VRF client is not set.")]
    VrfClientNotSet,
    #[msg("VRF callback has not timed out yet.")]
    VrfCallbackNotTimedOut,
    #[msg("Invalid fee percentage. Must be between 0 and 1000 basis points (10%).")]
    InvalidFeePercentage,
    #[msg("Invalid authority for this operation.")]
    InvalidAuthority,
    #[msg("No configuration changes provided.")]
    NoConfigChanges,
    #[msg("Too early to perform this action.")]
    TooEarly,
    #[msg("Invalid claim: ticket already claimed, not owned by signer, or not associated with lottery.")]
    InvalidClaim,
    #[msg("Invalid account provided.")]
    InvalidAccount,
}