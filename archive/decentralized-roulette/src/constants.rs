// Game configuration constants
pub const DEFAULT_TREASURY_FEE_PERCENTAGE: u16 = 200; // 2.00%
pub const DEFAULT_MIN_GAME_DURATION: i64 = 300; // 5 minutes
pub const DEFAULT_MAX_GAME_DURATION: i64 = 3600; // 1 hour
pub const DEFAULT_MIN_BET_AMOUNT: u64 = 1_000_000; // 1 USDC (in micro-lamports)
pub const DEFAULT_MAX_BET_AMOUNT: u64 = 1_000_000_000; // 1000 USDC
pub const DEFAULT_MAX_PLAYERS_PER_GAME: u64 = 1000;

// Game timing constants
pub const BETTING_DURATION: i64 = 180; // 3 minutes for betting
pub const LOCK_DURATION: i64 = 30; // 30 seconds lock period
pub const SPIN_DURATION: i64 = 30; // 30 seconds spinning
pub const REVEAL_DELAY: i64 = 60; // 1 minute total from betting end to reveal

// Roulette game constants
pub const EUROPEAN_ROULETTE_NUMBERS: u8 = 37; // 0-36
pub const AMERICAN_ROULETTE_NUMBERS: u8 = 38; // 0-36 + 00

// Bet type payout multipliers
pub const STRAIGHT_PAYOUT: u16 = 35; // 35:1
pub const SPLIT_PAYOUT: u16 = 17; // 17:1
pub const STREET_PAYOUT: u16 = 11; // 11:1
pub const CORNER_PAYOUT: u16 = 8; // 8:1
pub const SIXLINE_PAYOUT: u16 = 5; // 5:1
pub const EVEN_MONEY_PAYOUT: u16 = 1; // 1:1 (Red, Black, Even, Odd, Low, High)
pub const DOZEN_COLUMN_PAYOUT: u16 = 2; // 2:1 (Dozens and Columns)

// PDA seeds
pub const GLOBAL_CONFIG_SEED: &[u8] = b"global_config_v2";
pub const ROULETTE_SEED: &[u8] = b"roulette";
pub const BET_SEED: &[u8] = b"bet";
pub const ROULETTE_TOKEN_SEED: &[u8] = b"roulette_token";

// Validation constants
pub const MAX_BET_NUMBERS: usize = 38; // Maximum numbers a single bet can cover
pub const MIN_BET_NUMBERS: usize = 1; // Minimum numbers a bet must cover

// Red numbers in roulette (European/American)
pub const RED_NUMBERS: [u8; 18] = [1, 3, 5, 7, 9, 12, 14, 16, 18, 19, 21, 23, 25, 27, 30, 32, 34, 36];

// Black numbers in roulette (European/American)
pub const BLACK_NUMBERS: [u8; 18] = [2, 4, 6, 8, 10, 11, 13, 15, 17, 20, 22, 24, 26, 28, 29, 31, 33, 35];

// Column definitions
pub const FIRST_COLUMN: [u8; 12] = [1, 4, 7, 10, 13, 16, 19, 22, 25, 28, 31, 34];
pub const SECOND_COLUMN: [u8; 12] = [2, 5, 8, 11, 14, 17, 20, 23, 26, 29, 32, 35];
pub const THIRD_COLUMN: [u8; 12] = [3, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 36];

// VRF constants
pub const VRF_CLIENT_SEED: &[u8] = b"vrf_client";
pub const VRF_REQUEST_DELAY: i64 = 10; // Minimum seconds between request and consume

// Account size constants
pub const DISCRIMINATOR_SIZE: usize = 8;
pub const PUBKEY_SIZE: usize = 32;
pub const U64_SIZE: usize = 8;
pub const I64_SIZE: usize = 8;
pub const U16_SIZE: usize = 2;
pub const U8_SIZE: usize = 1;
pub const BOOL_SIZE: usize = 1;