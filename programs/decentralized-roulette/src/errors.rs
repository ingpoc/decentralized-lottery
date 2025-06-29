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
}