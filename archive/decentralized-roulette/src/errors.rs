use anchor_lang::prelude::*;

#[error_code]
pub enum RouletteError {
    #[msg("Invalid bet type")]
    InvalidBetType = 6000,
    
    #[msg("Bet amount below minimum")]
    BetBelowMinimum = 6001,
    
    #[msg("Bet amount exceeds maximum")]
    BetExceedsMaximum = 6002,
    
    #[msg("Betting period has ended")]
    BettingPeriodEnded = 6003,
    
    #[msg("Game is not in correct state")]
    InvalidGameState = 6004,
    
    #[msg("Invalid bet numbers for this bet type")]
    InvalidBetNumbers = 6005,
    
    #[msg("Maximum number of players reached")]
    MaxPlayersReached = 6006,
    
    #[msg("Randomness not yet fulfilled")]
    RandomnessNotFulfilled = 6007,
    
    #[msg("Winnings already claimed")]
    WinningsAlreadyClaimed = 6008,
    
    #[msg("Not a winning bet")]
    NotAWinningBet = 6009,
    
    #[msg("Game has expired")]
    GameExpired = 6010,
    
    #[msg("Game is paused")]
    GamePaused = 6011,
    
    #[msg("Invalid authority")]
    InvalidAuthority = 6012,
    
    #[msg("Invalid roulette type")]
    InvalidRouletteType = 6013,
    
    #[msg("Game duration too short")]
    GameDurationTooShort = 6014,
    
    #[msg("Game duration too long")]
    GameDurationTooLong = 6015,
    
    #[msg("Cannot transition to this state")]
    InvalidStateTransition = 6016,
    
    #[msg("VRF client not initialized")]
    VrfClientNotInitialized = 6017,
    
    #[msg("Invalid VRF account")]
    InvalidVrfAccount = 6018,
    
    #[msg("Arithmetic overflow")]
    ArithmeticOverflow = 6019,
    
    #[msg("Insufficient funds")]
    InsufficientFunds = 6020,
    
    #[msg("Invalid token account")]
    InvalidTokenAccount = 6021,
    
    #[msg("Too early to perform this action")]
    TooEarly = 6022,
    
    #[msg("Too late to perform this action")]
    TooLate = 6023,
    
    #[msg("Invalid nonce")]
    InvalidNonce = 6024,
    
    #[msg("Cannot place duplicate bets")]
    DuplicateBet = 6025,
    
    #[msg("Invalid bet amount")]
    InvalidBetAmount = 6026,
    
    #[msg("Betting is closed")]
    BettingClosed = 6027,
    
    #[msg("Game is full - maximum players reached")]
    GameFull = 6028,
    
    #[msg("Too many bets - potential DoS attack")]
    TooManyBets = 6029,
    
    #[msg("Randomness not ready")]
    RandomnessNotReady = 6030,
    
    #[msg("Invalid randomness data")]
    InvalidRandomness = 6031,
    
    #[msg("Clock error")]
    ClockError = 6032,
    
    #[msg("Randomness already fulfilled")]
    RandomnessAlreadyFulfilled = 6033,

    #[msg("VRF request failed")]
    VrfRequestFailed = 6034,

    #[msg("VRF timeout")]
    VrfTimeout = 6035,

    #[msg("Invalid treasury account")]
    InvalidTreasuryAccount = 6036,

    #[msg("Game is stuck and needs manual intervention")]
    GameStuck = 6037,

    #[msg("Invalid time configuration")]
    InvalidTimeConfiguration = 6038,

    #[msg("Treasury has insufficient funds for potential payout")]
    InsufficientTreasury = 6039,

    #[msg("Bet too large relative to treasury capacity")]
    BetTooLarge = 6040,

    #[msg("Invalid VRF program")]
    InvalidVrfProgram = 6041,

    #[msg("VRF request not authorized")]
    VrfRequestUnauthorized = 6042,
    
    #[msg("Invalid account provided")]
    InvalidAccount = 6043,
    
    #[msg("Payout calculation overflow")]
    PayoutOverflow = 6044,
    
    #[msg("Treasury fee calculation overflow")]
    TreasuryFeeOverflow = 6045,
    
    #[msg("Randomness not available")]
    RandomnessNotAvailable = 6046,
    
    #[msg("Randomness generation failed")]
    RandomnessGenerationFailed = 6047,
}
