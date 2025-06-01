import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';

// Configuration
const PROJECT_ROOT = path.resolve(__dirname, '..');
const TARGET_DIR = path.join(PROJECT_ROOT, 'target');
const IDL_DIR = path.join(TARGET_DIR, 'idl');
const TYPES_DIR = path.join(TARGET_DIR, 'types');
const FRONTEND_DIR = path.join(PROJECT_ROOT, '..', 'stockanalysisgui');
const FRONTEND_LIB_DIR = path.join(FRONTEND_DIR, 'src', 'lib', 'solana');
const FRONTEND_TYPES_DIR = path.join(FRONTEND_DIR, 'src', 'types');

// File paths
const IDL_FILE = path.join(IDL_DIR, 'decentralized_lottery.json');
const TYPES_FILE = path.join(TYPES_DIR, 'decentralized_lottery.ts');
const FRONTEND_IDL_FILE = path.join(FRONTEND_LIB_DIR, 'decentralized_lottery.json');
const FRONTEND_AUTO_TYPES_FILE = path.join(FRONTEND_TYPES_DIR, 'decentralized_lottery.ts');
const FRONTEND_TYPES_FILE = path.join(FRONTEND_TYPES_DIR, 'lottery_types.ts');

/**
 * Ensures a directory exists, creating it if necessary
 */
function ensureDirectoryExists(dirPath: string): void {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`Created directory: ${dirPath}`);
  }
}

/**
 * Runs the anchor idl type command to generate TypeScript types
 */
function generateTypes(): void {
  try {
    console.log('Generating TypeScript types from IDL...');
    execSync(`anchor idl type -o ${TYPES_FILE} ${IDL_FILE}`, { 
      cwd: PROJECT_ROOT,
      stdio: 'inherit'
    });
    console.log('TypeScript types generated successfully.');
  } catch (error) {
    console.error('Error generating TypeScript types:', error);
    process.exit(1);
  }
}

/**
 * Copies the IDL file to the frontend directory
 */
function copyIdlToFrontend(): void {
  try {
    console.log(`Copying IDL file to frontend: ${FRONTEND_IDL_FILE}`);
    ensureDirectoryExists(path.dirname(FRONTEND_IDL_FILE));
    fs.copyFileSync(IDL_FILE, FRONTEND_IDL_FILE);
    console.log('IDL file copied successfully.');
  } catch (error) {
    console.error('Error copying IDL file:', error);
    process.exit(1);
  }
}

/**
 * Copies the auto-generated types to frontend for reference
 */
function copyAutoTypesToFrontend(): void {
  try {
    console.log(`Copying auto-generated types to frontend: ${FRONTEND_AUTO_TYPES_FILE}`);
    ensureDirectoryExists(path.dirname(FRONTEND_AUTO_TYPES_FILE));
    
    // Read the generated types file
    let typesContent = fs.readFileSync(TYPES_FILE, 'utf8');
    
    // Transform the content for frontend use
    typesContent = typesContent
      // Add a comment header
      .replace(
        'export type DecentralizedLottery', 
        '/**\n * Auto-generated Program IDL types in camelCase format.\n *\n * Note that this is auto-generated from the IDL. For comprehensive\n * types with UI helpers, use lottery_types.ts instead.\n */\nexport type DecentralizedLottery'
      );
    
    // Write the transformed content to the frontend auto-types file
    fs.writeFileSync(FRONTEND_AUTO_TYPES_FILE, typesContent);
    console.log('Auto-generated types copied successfully.');
  } catch (error) {
    console.error('Error copying auto-generated types:', error);
    process.exit(1);
  }
}

/**
 * Creates or updates the comprehensive lottery types file
 */
function createComprehensiveLotteryTypes(): void {
  try {
    console.log(`Creating comprehensive lottery types: ${FRONTEND_TYPES_FILE}`);
    ensureDirectoryExists(path.dirname(FRONTEND_TYPES_FILE));
    
    // Read the IDL to extract current program information
    const idlContent = JSON.parse(fs.readFileSync(IDL_FILE, 'utf8'));
    const programId = idlContent.address;
    const version = idlContent.metadata.version;
    
    // Generate comprehensive types content
    const comprehensiveTypes = `/**
 * Comprehensive Lottery Types
 * 
 * This file contains UI-friendly types, helpers, and utilities for the
 * decentralized lottery program. Auto-generated types are available
 * in decentralized_lottery.ts.
 */

import { PublicKey } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';

// Re-export auto-generated types for convenience
export type { DecentralizedLottery } from './decentralized_lottery';

// ===== ENUMS =====

export type LotteryState = 
  | 'Created'
  | 'Open' 
  | 'Locked'
  | 'Drawing'
  | 'AwaitingRandomness'
  | 'Completed'
  | 'Expired'
  | 'Cancelled';

export type LotteryType = 
  | 'Daily'
  | 'Weekly'
  | 'Monthly';

// ===== CORE ACCOUNT TYPES =====

export interface GlobalConfig {
  admin: PublicKey;
  treasuryTokenAccount: PublicKey;
  treasuryFeePercentage: number;
  usdcMint: PublicKey;
}

export interface LotteryAccount {
  lotteryType: LotteryType;
  ticketPrice: BN;
  drawTime: BN;
  prizePool: BN;
  totalTickets: BN;
  winningTicket: PublicKey | null;
  state: LotteryState;
  createdBy: PublicKey;
  globalConfig: PublicKey;
  autoTransition: boolean;
  lastTicketId: BN;
  authority: PublicKey;
  vrfClient: PublicKey | null;
  vrfRandomness: Uint8Array | null;
  vrfRequestAccount: PublicKey | null;
  oraclePublickey: PublicKey | null;
  isPrizePoolLocked: boolean;
  targetPrizePool: BN;
  isClaimed: boolean;
  createdAt: BN;
  completedAt: BN | null;
}

export interface TicketAccount {
  lotteryId: PublicKey;
  ticketId: BN;
  owner: PublicKey;
  isClaimed: boolean;
  purchasedAt: BN;
}

// ===== EVENT TYPES =====

export interface DrawingStarted {
  lotteryId: PublicKey;
  timestamp: BN;
  totalTickets: BN;
  prizePool: BN;
  vrfClient: PublicKey | null;
}

export interface LotteryCreated {
  lotteryId: PublicKey;
  lotteryType: string;
  ticketPrice: BN;
  drawTime: BN;
  targetPrizePool: BN;
}

export interface LotteryStateChanged {
  lotteryId: PublicKey;
  previousState: LotteryState;
  newState: LotteryState;
  timestamp: BN;
  totalTicketsSold: BN;
  currentPrizePool: BN;
}

export interface PrizeClaimed {
  lotteryId: PublicKey;
  winner: PublicKey;
  prizeAmount: BN;
  treasuryFee: BN;
  timestamp: BN;
}

export interface RandomnessConsumed {
  lotteryId: PublicKey;
  randomness: Uint8Array;
  selectedTicketIndex: BN;
  timestamp: BN;
}

export interface RandomnessRequested {
  lotteryId: PublicKey;
  vrfClient: PublicKey;
  requestAccount: PublicKey;
  timestamp: BN;
}

export interface TicketPurchased {
  lotteryId: PublicKey;
  ticketId: BN;
  purchaser: PublicKey;
  ticketPrice: BN;
  timestamp: BN;
}

export interface TicketRefunded {
  lotteryId: PublicKey;
  ticketId: BN;
  refundRecipient: PublicKey;
  refundAmount: BN;
  timestamp: BN;
}

export interface TreasuryWithdrawal {
  recipient: PublicKey;
  amount: BN;
  timestamp: BN;
}

export interface VrfClientInitialized {
  vrfClient: PublicKey;
  authority: PublicKey;
  timestamp: BN;
}

export interface WinnerSelected {
  lotteryId: PublicKey;
  winningTicket: PublicKey;
  winner: PublicKey;
  prizeAmount: BN;
  timestamp: BN;
}

// ===== INSTRUCTION ARGUMENTS =====

export interface CreateLotteryArgs {
  lotteryTypeEnum: LotteryType;
  ticketPrice: BN;
  drawTime: BN;
  targetPrizePool: BN;
}

export interface TransitionStateArgs {
  nextState: LotteryState;
}

// ===== UI INTERFACE TYPES =====

export interface LotteryInfo {
  id: PublicKey;
  account: LotteryAccount;
  // Display-friendly properties
  ticketPriceDisplay: number;
  prizePoolDisplay: number;
  targetPrizePoolDisplay: number;
  drawTimeDisplay: Date;
  createdAtDisplay: Date;
  completedAtDisplay: Date | null;
  totalTicketsDisplay: number;
  lastTicketIdDisplay: number;
  winningNumbers?: number[];
  globalConfig?: GlobalConfig;
}

export interface TicketInfo {
  id: PublicKey;
  account: TicketAccount;
  // Display-friendly properties
  ticketIdDisplay: number;
  purchasedAtDisplay: Date;
}

// ===== ERROR TYPES =====

export enum LotteryErrorCode {
  UnsupportedLotteryType = 6000,
  InvalidTicketPrice = 6001,
  InvalidPrizePool = 6002,
  InvalidDrawTime = 6003,
  InvalidTicketAmount = 6004,
  TicketPurchaseLimitReached = 6005,
  LotteryNotOpen = 6006,
  LotteryDrawing = 6007,
  LotteryCompleted = 6008,
  LotteryExpired = 6009,
  InvalidLotteryState = 6010,
  InvalidAccountOwner = 6011,
  InvalidInstructionInput = 6012,
  SafeMathError = 6013,
  PrizeClaimTimeExpired = 6014,
  InvalidPrizeTier = 6015,
  TreasuryWithdrawalTimeLockNotReached = 6016,
  InvalidTreasuryMultisig = 6017,
  TokenTransferFailed = 6018,
  InvalidTokenAccount = 6019,
  InvalidTokenMint = 6020,
  OraclePriceFeedError = 6021,
  RandomnessGenerationFailed = 6022,
  UnauthorizedAccess = 6023,
  InvalidStateTransition = 6024,
  InvalidCancellation = 6025,
  AdminRequired = 6026,
  LotteryCancelled = 6027,
  LotteryNotOpenForTicketPurchases = 6028,
  LotteryAlreadyClaimed = 6029,
  PDADerivationError = 6030,
  InvalidWinningTicket = 6031,
  TicketAlreadyClaimed = 6032,
  InvalidStateForRefund = 6033,
  InvalidInput = 6034,
  TicketSaleEnded = 6035,
  LotteryAlreadyDrawn = 6036,
  NoTickets = 6037,
  InsufficientTicketsSold = 6038,
  LotteryNotDrawn = 6039,
  TicketNotEligibleForRefund = 6040,
  LotteryNotExpired = 6041,
  InvalidVrfAccount = 6042,
  InsufficientFunds = 6043,
  ArithmeticOverflow = 6044,
  TicketNotForThisLottery = 6045,
  NoWinnerSelected = 6046,
  EmptyPrizePool = 6047,
  InsufficientPrizeFunds = 6048
}

export interface LotteryError {
  code: LotteryErrorCode;
  name: string;
  message: string;
}

// ===== CONSTANTS =====

export const PROGRAM_ID = new PublicKey('${programId}');

export const LOTTERY_SEED = 'lottery';
export const GLOBAL_CONFIG_SEED = 'global_config';
export const TICKET_SEED = 'ticket';

// USDC has 6 decimal places
export const USDC_DECIMALS = 6;

// ===== HELPER TYPES =====

export interface PDASeeds {
  lottery: string;
  globalConfig: string;
  ticket: string;
}

export const PDA_SEEDS: PDASeeds = {
  lottery: LOTTERY_SEED,
  globalConfig: GLOBAL_CONFIG_SEED,
  ticket: TICKET_SEED
};

// ===== STATE TRANSITION HELPERS =====

export const VALID_STATE_TRANSITIONS: Record<LotteryState, LotteryState[]> = {
  'Created': ['Open', 'Cancelled'],
  'Open': ['Locked', 'Drawing', 'Expired', 'Cancelled'],
  'Locked': ['Drawing', 'Expired', 'Cancelled'],
  'Drawing': ['AwaitingRandomness', 'Completed', 'Expired'],
  'AwaitingRandomness': ['Completed', 'Expired'],
  'Completed': [],
  'Expired': [],
  'Cancelled': []
};

export function isValidStateTransition(current: LotteryState, next: LotteryState): boolean {
  return VALID_STATE_TRANSITIONS[current].includes(next);
}

// ===== DISPLAY HELPERS =====

export function formatUSDC(amount: BN | number): number {
  const amountNum = typeof amount === 'number' ? amount : amount.toNumber();
  return amountNum / Math.pow(10, USDC_DECIMALS);
}

export function parseUSDC(amount: number): BN {
  return new BN(amount * Math.pow(10, USDC_DECIMALS));
}

export function formatTimestamp(timestamp: BN): Date {
  return new Date(timestamp.toNumber() * 1000);
}

export function parseTimestamp(date: Date): BN {
  return new BN(Math.floor(date.getTime() / 1000));
}

// ===== LOTTERY STATE HELPERS =====

export function getLotteryStateColor(state: LotteryState): string {
  switch (state) {
    case 'Created': return 'gray';
    case 'Open': return 'green';
    case 'Locked': return 'yellow';
    case 'Drawing': return 'blue';
    case 'AwaitingRandomness': return 'blue';
    case 'Completed': return 'purple';
    case 'Expired': return 'red';
    case 'Cancelled': return 'red';
    default: return 'gray';
  }
}

export function getLotteryStateDescription(state: LotteryState): string {
  switch (state) {
    case 'Created': return 'Lottery has been created but not yet opened for tickets';
    case 'Open': return 'Tickets can be purchased';
    case 'Locked': return 'Ticket sales have ended, awaiting draw';
    case 'Drawing': return 'Draw is in progress';
    case 'AwaitingRandomness': return 'Waiting for VRF randomness';
    case 'Completed': return 'Draw completed, winner selected';
    case 'Expired': return 'Lottery has expired without completion';
    case 'Cancelled': return 'Lottery was cancelled';
    default: return 'Unknown state';
  }
}

export function canPurchaseTickets(state: LotteryState): boolean {
  return state === 'Open';
}

export function canClaim(state: LotteryState): boolean {
  return state === 'Completed';
}

export function canRefund(state: LotteryState): boolean {
  return state === 'Cancelled' || state === 'Expired';
}

// ===== LOTTERY TYPE HELPERS =====

export function getLotteryTypeDescription(type: LotteryType): string {
  switch (type) {
    case 'Daily': return 'Daily lottery - draws every 24 hours';
    case 'Weekly': return 'Weekly lottery - draws every 7 days';
    case 'Monthly': return 'Monthly lottery - draws every 30 days';
    default: return 'Unknown lottery type';
  }
}

export function getLotteryTypeDuration(type: LotteryType): number {
  switch (type) {
    case 'Daily': return 24 * 60 * 60; // 24 hours in seconds
    case 'Weekly': return 7 * 24 * 60 * 60; // 7 days in seconds
    case 'Monthly': return 30 * 24 * 60 * 60; // 30 days in seconds
    default: return 0;
  }
}

// ===== IDL REFERENCE =====

export const IDL_ADDRESS = PROGRAM_ID.toBase58();
export const IDL_VERSION = '${version}';

// For backward compatibility with existing code
export type { LotteryAccount as Lottery };
export type { TicketAccount as Ticket };
`;

    // Write the comprehensive types file
    fs.writeFileSync(FRONTEND_TYPES_FILE, comprehensiveTypes);
    console.log('Comprehensive lottery types created successfully.');
  } catch (error) {
    console.error('Error creating comprehensive lottery types:', error);
    process.exit(1);
  }
}

/**
 * Main function to run the script
 */
function main(): void {
  console.log('Starting IDL and types update process...');
  
  // Check if IDL file exists
  if (!fs.existsSync(IDL_FILE)) {
    console.error(`IDL file not found: ${IDL_FILE}`);
    console.error('Please run "anchor build" first to generate the IDL.');
    process.exit(1);
  }
  
  // Generate TypeScript types
  generateTypes();
  
  // Copy files to frontend
  copyIdlToFrontend();
  copyAutoTypesToFrontend();
  createComprehensiveLotteryTypes();
  
  console.log('IDL and types update completed successfully!');
  console.log('Files generated:');
  console.log(`  - Auto-generated types: ${path.relative(process.cwd(), FRONTEND_AUTO_TYPES_FILE)}`);
  console.log(`  - Comprehensive types: ${path.relative(process.cwd(), FRONTEND_TYPES_FILE)}`);
  console.log(`  - IDL file: ${path.relative(process.cwd(), FRONTEND_IDL_FILE)}`);
}

// Run the script
main(); 