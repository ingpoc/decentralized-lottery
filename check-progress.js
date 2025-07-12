#!/usr/bin/env node

/**
 * Check Progress of Stuck Games Processing
 */

const { Connection, PublicKey } = require('@solana/web3.js');

// Configuration
const PROGRAM_ID = new PublicKey('4ZVg5wU59Tr6pKAfxkTFsF2cffGrVRM2xqt1WbPUJrUB');
const DEVNET_URL = 'https://api.devnet.solana.com';

// Known stuck games
const stuckGames = [
  'Hp7k528p1ZhqQokXxkX89uf98J5QV1UJhxu25XptEGrj',
  '6MvMv1SeUqxPmo32SvtG5J8rrGHcQJ6edeDrSktUDjLX',
  'BGbEaXUm4MvbFEiZVumAMS1NqEYBTS3rDMBB9Aoqrzo2',
  '3wugFAwVfGR6BqujcnbFmGnXNAseCM5TGsCdrNKXaANp',
  'CKzipEbtKn5Rmt8Q6K88vq35r8i5HyUGk7Jm83LWpWPc',
];

function decodeRouletteState(data) {
  try {
    // Read state from position 81 (based on account structure)
    const stateByte = data[81];
    const states = ['Created', 'Open', 'Locked', 'Spinning', 'AwaitingRandomness', 'Completed', 'Expired', 'Cancelled'];
    const state = states[stateByte] || `Unknown(${stateByte})`;
    
    // Read timestamps (8 bytes each, little-endian)
    const startTime = Number(data.readBigInt64LE(33));
    const bettingEndTime = Number(data.readBigInt64LE(41));
    const endTime = Number(data.readBigInt64LE(65));
    
    return {
      state,
      startTime,
      bettingEndTime, 
      endTime,
      stateByte
    };
  } catch (error) {
    return { state: 'DecodeError', error: error.message };
  }
}

async function checkProgress() {
  console.log('📊 CHECKING STUCK GAMES PROGRESS');
  console.log('===============================\n');
  
  const connection = new Connection(DEVNET_URL, 'confirmed');
  const now = Math.floor(Date.now() / 1000);
  
  console.log('🔍 Checking specific stuck games...\n');
  
  let stillStuck = 0;
  let processed = 0;
  let errors = 0;
  
  for (let i = 0; i < stuckGames.length; i++) {
    const gameAddress = new PublicKey(stuckGames[i]);
    
    try {
      const accountInfo = await connection.getAccountInfo(gameAddress);
      
      if (!accountInfo) {
        console.log(`❌ Game ${i + 1}: ${stuckGames[i].slice(0, 8)}... - Account not found`);
        errors++;
        continue;
      }
      
      const state = decodeRouletteState(accountInfo.data);
      const hoursAgo = Math.floor((now - state.endTime) / 3600);
      
      console.log(`🎮 Game ${i + 1}: ${stuckGames[i].slice(0, 8)}...`);
      console.log(`   State: ${state.state} (byte: ${state.stateByte})`);
      console.log(`   End Time: ${new Date(state.endTime * 1000).toLocaleString()}`);
      console.log(`   Hours Past End: ${hoursAgo}h`);
      
      if (state.state === 'Open' && state.endTime < now) {
        console.log(`   🔴 Still stuck - needs processing`);
        stillStuck++;
      } else if (state.state === 'Expired' || state.state === 'Completed') {
        console.log(`   ✅ Processed successfully`);
        processed++;
      } else {
        console.log(`   ⚠️  Unknown state: ${state.state}`);
      }
      
      console.log();
      
      // Add delay to avoid rate limiting
      await new Promise(resolve => setTimeout(resolve, 200));
      
    } catch (error) {
      console.log(`❌ Game ${i + 1}: ${stuckGames[i].slice(0, 8)}... - Error: ${error.message}`);
      errors++;
    }
  }
  
  console.log('📈 SUMMARY:');
  console.log(`   ✅ Successfully processed: ${processed} games`);
  console.log(`   🔴 Still stuck: ${stillStuck} games`);
  console.log(`   ❌ Errors: ${errors} games`);
  console.log(`   📊 Total checked: ${stuckGames.length} games`);
  
  if (processed > 0) {
    console.log('\n🎉 PROGRESS DETECTED!');
    console.log(`   ${processed} games have been successfully processed`);
    console.log('   The frontend processGameLifecycle function is working!');
  }
  
  if (stillStuck > 0) {
    console.log('\n🔧 REMAINING WORK:');
    console.log(`   ${stillStuck} games still need processing`);
    console.log('   Continue using the admin interface to process them');
  }
  
  if (processed === stuckGames.length) {
    console.log('\n🏆 ALL GAMES PROCESSED!');
    console.log('   Ready to create new games and test automation');
  }
}

checkProgress().catch(console.error);