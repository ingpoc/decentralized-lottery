#!/usr/bin/env node

/**
 * Debug Roulette State Decoder
 * 
 * This script accurately decodes roulette account data to understand
 * the current state and timing values.
 */

const { Connection, PublicKey } = require('@solana/web3.js');

// Configuration
const PROGRAM_ID = '4ZVg5wU59Tr6pKAfxkTFsF2cffGrVRM2xqt1WbPUJrUB';
const DEVNET_URL = 'https://api.devnet.solana.com';
const TARGET_GAME = 'Hp7k528p1ZhqQokXxkX89uf98J5QV1UJhxu25XptEGrj';

// Accurate decoder based on RouletteAccount structure
function decodeRouletteAccount(data) {
  if (!data || data.length < 363) {
    throw new Error('Invalid roulette account data');
  }

  let offset = 0;
  
  // Skip discriminator (8 bytes)
  offset += 8;
  
  // RouletteType enum (1 byte)
  const rouletteType = data[offset];
  offset += 1;
  
  // Min bet (u64, 8 bytes)
  const minBet = data.readBigUInt64LE(offset);
  offset += 8;
  
  // Max bet (u64, 8 bytes)
  const maxBet = data.readBigUInt64LE(offset);
  offset += 8;
  
  // Game duration (i64, 8 bytes)
  const gameDuration = data.readBigInt64LE(offset);
  offset += 8;
  
  // Betting duration (i64, 8 bytes)
  const bettingDuration = data.readBigInt64LE(offset);
  offset += 8;
  
  // Start time (i64, 8 bytes)
  const startTime = data.readBigInt64LE(offset);
  offset += 8;
  
  // Betting end time (i64, 8 bytes)
  const bettingEndTime = data.readBigInt64LE(offset);
  offset += 8;
  
  // Spin time (i64, 8 bytes)
  const spinTime = data.readBigInt64LE(offset);
  offset += 8;
  
  // Reveal time (i64, 8 bytes)
  const revealTime = data.readBigInt64LE(offset);
  offset += 8;
  
  // End time (i64, 8 bytes)
  const endTime = data.readBigInt64LE(offset);
  offset += 8;
  
  // State enum (1 byte)
  const state = data[offset];
  offset += 1;
  
  // Total bets (u64, 8 bytes)
  const totalBets = data.readBigUInt64LE(offset);
  offset += 8;
  
  // Total bet amount (u64, 8 bytes)
  const totalBetAmount = data.readBigUInt64LE(offset);
  offset += 8;
  
  // Total players (u64, 8 bytes)
  const totalPlayers = data.readBigUInt64LE(offset);
  offset += 8;
  
  // Winning number (Option<u8>, 1 + 1 bytes)
  const hasWinningNumber = data[offset] === 1;
  offset += 1;
  const winningNumber = hasWinningNumber ? data[offset] : null;
  offset += 1;
  
  // State names
  const stateNames = [
    'Created', 'Open', 'Locked', 'Spinning', 
    'AwaitingRandomness', 'Completed', 'Expired', 'Cancelled'
  ];
  
  const rouletteTypeNames = ['European', 'American'];
  
  return {
    rouletteType: rouletteTypeNames[rouletteType] || `Unknown(${rouletteType})`,
    minBet: Number(minBet),
    maxBet: Number(maxBet),
    gameDuration: Number(gameDuration),
    bettingDuration: Number(bettingDuration),
    startTime: Number(startTime),
    bettingEndTime: Number(bettingEndTime),
    spinTime: Number(spinTime),
    revealTime: Number(revealTime),
    endTime: Number(endTime),
    state: stateNames[state] || `Unknown(${state})`,
    totalBets: Number(totalBets),
    totalBetAmount: Number(totalBetAmount),
    totalPlayers: Number(totalPlayers),
    winningNumber,
    hasWinningNumber
  };
}

async function main() {
  console.log('🔍 ROULETTE STATE DEBUGGER');
  console.log('=========================\n');
  
  const connection = new Connection(DEVNET_URL, 'confirmed');
  const gameAddress = new PublicKey(TARGET_GAME);
  
  try {
    // Get the account info
    const accountInfo = await connection.getAccountInfo(gameAddress);
    if (!accountInfo) {
      console.log('❌ Account not found');
      return;
    }
    
    console.log('📋 Raw Account Info:');
    console.log(`   Address: ${TARGET_GAME}`);
    console.log(`   Data Length: ${accountInfo.data.length} bytes`);
    console.log(`   Lamports: ${accountInfo.lamports}`);
    console.log(`   Owner: ${accountInfo.owner.toString()}\n`);
    
    // Decode the account data
    const decoded = decodeRouletteAccount(accountInfo.data);
    
    console.log('🎰 Decoded Roulette Account:');
    console.log(`   Type: ${decoded.rouletteType}`);
    console.log(`   State: ${decoded.state}`);
    console.log(`   Min Bet: ${decoded.minBet / 1_000_000} USDC`);
    console.log(`   Max Bet: ${decoded.maxBet / 1_000_000} USDC`);
    console.log(`   Game Duration: ${decoded.gameDuration}s`);
    console.log(`   Betting Duration: ${decoded.bettingDuration}s`);
    console.log();
    
    console.log('⏰ Timing Analysis:');
    const now = Math.floor(Date.now() / 1000);
    console.log(`   Current Time: ${now} (${new Date(now * 1000).toISOString()})`);
    console.log(`   Start Time: ${decoded.startTime} (${new Date(decoded.startTime * 1000).toISOString()})`);
    console.log(`   Betting End: ${decoded.bettingEndTime} (${new Date(decoded.bettingEndTime * 1000).toISOString()})`);
    console.log(`   Spin Time: ${decoded.spinTime} (${new Date(decoded.spinTime * 1000).toISOString()})`);
    console.log(`   Reveal Time: ${decoded.revealTime} (${new Date(decoded.revealTime * 1000).toISOString()})`);
    console.log(`   End Time: ${decoded.endTime} (${new Date(decoded.endTime * 1000).toISOString()})`);
    console.log();
    
    console.log('🎯 Game Statistics:');
    console.log(`   Total Bets: ${decoded.totalBets}`);
    console.log(`   Total Bet Amount: ${decoded.totalBetAmount / 1_000_000} USDC`);
    console.log(`   Total Players: ${decoded.totalPlayers}`);
    console.log(`   Winning Number: ${decoded.winningNumber || 'Not set'}`);
    console.log();
    
    // Time analysis
    const timeSinceStart = now - decoded.startTime;
    const timeSinceBettingEnd = now - decoded.bettingEndTime;
    const timeSinceSpin = now - decoded.spinTime;
    const timeSinceEnd = now - decoded.endTime;
    
    console.log('⏱️  Time Differences:');
    console.log(`   Time Since Start: ${timeSinceStart}s (${Math.floor(timeSinceStart / 60)}m)`);
    console.log(`   Time Since Betting End: ${timeSinceBettingEnd}s (${Math.floor(timeSinceBettingEnd / 60)}m)`);
    console.log(`   Time Since Spin: ${timeSinceSpin}s (${Math.floor(timeSinceSpin / 60)}m)`);
    console.log(`   Time Since End: ${timeSinceEnd}s (${Math.floor(timeSinceEnd / 60)}m)`);
    console.log();
    
    // State validation
    console.log('✅ State Validation:');
    if (decoded.state === 'Open' && timeSinceBettingEnd > 0) {
      console.log('   ❌ BUG: Game should be Locked (betting period ended)');
    } else if (decoded.state === 'Locked' && timeSinceSpin > 0) {
      console.log('   ❌ BUG: Game should be Spinning/Completed (spin time passed)');
    } else if (decoded.state === 'Spinning' && timeSinceSpin > 30) {
      console.log('   ❌ BUG: Game should be Completed (been spinning too long)');
    } else if ((decoded.state === 'Open' || decoded.state === 'Locked') && timeSinceEnd > 0) {
      console.log('   ❌ BUG: Game should be Expired (end time passed)');
    } else {
      console.log('   ✅ Game state appears correct for current timing');
    }
    
    // Check if automatic transitions should have happened
    console.log('\n🔄 Expected State Transitions:');
    if (decoded.totalBets > 0) {
      console.log('   ✅ Bets placed - automatic lifecycle should have been triggered');
      
      if (timeSinceBettingEnd > 0) {
        console.log('   Expected: Open → Locked');
      }
      if (timeSinceSpin > 0) {
        console.log('   Expected: Locked → Spinning → Completed');
      }
      if (timeSinceEnd > 0) {
        console.log('   Expected: Any state → Expired');
      }
    } else {
      console.log('   ⚠️  No bets placed - automatic lifecycle not triggered');
    }
    
  } catch (error) {
    console.error('❌ Error:', error.message);
    console.error('Stack:', error.stack);
  }
}

main().catch(console.error);