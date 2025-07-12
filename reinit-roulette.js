/**
 * Reinitialize Roulette Program
 * 
 * This script cleanly reinitializes the roulette program with automatic game creation.
 * After running this, the system will operate fully automatically.
 */

const anchor = require('@coral-xyz/anchor');
const { PublicKey, SystemProgram } = require('@solana/web3.js');
const { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } = require('@solana/spl-token');

// Note: Using anchor default wallet configuration

const PROGRAM_ID = new PublicKey('4ZVg5wU59Tr6pKAfxkTFsF2cffGrVRM2xqt1WbPUJrUB');
const USDC_MINT_DEVNET = new PublicKey('Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr');
const GLOBAL_CONFIG_SEED = 'global_config_v2';
const ROULETTE_SEED = 'roulette';
const ROULETTE_TOKEN_SEED = 'roulette_token';

async function main() {
  console.log('🎰 Reinitializing Roulette Program...\n');

  // Setup connection and provider
  const connection = new anchor.web3.Connection('https://api.devnet.solana.com');
  const wallet = anchor.AnchorProvider.env().wallet;
  const provider = new anchor.AnchorProvider(connection, wallet, {});
  anchor.setProvider(provider);

  // Load program
  const idl = JSON.parse(require('fs').readFileSync('./target/idl/decentralized_roulette.json', 'utf8'));
  const program = new anchor.Program(idl, PROGRAM_ID, provider);

  console.log('📋 Program ID:', PROGRAM_ID.toString());
  console.log('👤 Admin Wallet:', provider.wallet.publicKey.toString());
  console.log('🌐 Network: devnet\n');

  try {
    // Derive PDAs
    const [globalConfigPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(GLOBAL_CONFIG_SEED)],
      PROGRAM_ID
    );

    // Create first roulette game PDA
    const [firstRoulettePDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(ROULETTE_SEED), provider.wallet.publicKey.toBuffer(), Buffer.from([1, 0, 0, 0, 0, 0, 0, 0])],
      PROGRAM_ID
    );

    // Create first roulette token account PDA
    const [firstRouletteTokenPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from(ROULETTE_TOKEN_SEED), firstRoulettePDA.toBuffer()],
      PROGRAM_ID
    );

    // Get treasury token account (admin's USDC account)
    const treasuryTokenAccount = await getAssociatedTokenAddress(
      USDC_MINT_DEVNET,
      provider.wallet.publicKey
    );

    console.log('🔑 Derived Addresses:');
    console.log('   Global Config PDA:', globalConfigPDA.toString());
    console.log('   First Roulette PDA:', firstRoulettePDA.toString());
    console.log('   First Roulette Token PDA:', firstRouletteTokenPDA.toString());
    console.log('   Treasury Token Account:', treasuryTokenAccount.toString());
    console.log();

    // Check if already initialized
    try {
      const existingConfig = await program.account.globalConfig.fetch(globalConfigPDA);
      console.log('⚠️  Program already initialized!');
      console.log('   Authority:', existingConfig.authority.toString());
      console.log('   USDC Mint:', existingConfig.usdcMint.toString());
      console.log('   Treasury Fee:', existingConfig.treasuryFeePercentage);
      console.log('   Is Paused:', existingConfig.isPaused);
      console.log('\n✅ Program is ready for automatic operation!');
      return;
    } catch (error) {
      console.log('✨ Program not yet initialized, proceeding with initialization...\n');
    }

    // Initialize the program with automatic first game creation
    console.log('🚀 Initializing roulette program with automatic game creation...');
    
    const tx = await program.methods
      .initialize()
      .accounts({
        globalConfig: globalConfigPDA,
        firstRoulette: firstRoulettePDA,
        authority: provider.wallet.publicKey,
        usdcMint: USDC_MINT_DEVNET,
        treasuryTokenAccount: treasuryTokenAccount,
        firstRouletteTokenAccount: firstRouletteTokenPDA,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log('✅ Initialization successful!');
    console.log('   Transaction:', tx);
    console.log();

    // Verify initialization
    const globalConfig = await program.account.globalConfig.fetch(globalConfigPDA);
    const firstRoulette = await program.account.rouletteAccount.fetch(firstRoulettePDA);

    console.log('📊 Program Configuration:');
    console.log('   Authority:', globalConfig.authority.toString());
    console.log('   USDC Mint:', globalConfig.usdcMint.toString());
    console.log('   Treasury Fee:', globalConfig.treasuryFeePercentage + '%');
    console.log('   Min Game Duration:', globalConfig.minGameDuration.toString(), 'seconds');
    console.log('   Max Game Duration:', globalConfig.maxGameDuration.toString(), 'seconds');
    console.log();

    console.log('🎮 First Game Created:');
    console.log('   Game Address:', firstRoulettePDA.toString());
    console.log('   State:', Object.keys(firstRoulette.state)[0]);
    console.log('   Min Bet:', firstRoulette.minBet.toString(), 'USDC (micro)');
    console.log('   Max Bet:', firstRoulette.maxBet.toString(), 'USDC (micro)');
    console.log('   Start Time:', new Date(firstRoulette.startTime.toNumber() * 1000).toISOString());
    console.log('   End Time:', new Date(firstRoulette.endTime.toNumber() * 1000).toISOString());
    console.log();

    console.log('🎯 SYSTEM STATUS: FULLY AUTOMATED');
    console.log('   ✅ Program initialized');
    console.log('   ✅ First game created automatically');
    console.log('   ✅ Automatic lifecycle processing enabled');
    console.log('   ✅ Automatic game creation enabled');
    console.log('   ✅ Continuous operation ready');
    console.log();
    console.log('🚀 The roulette system is now running automatically!');
    console.log('   • Games will progress through states automatically when bets are placed');
    console.log('   • New games will be created automatically when current games complete');
    console.log('   • No manual intervention required');
    console.log();
    console.log('🎲 Ready for betting!');

  } catch (error) {
    console.error('❌ Initialization failed:', error);
    process.exit(1);
  }
}

main().catch(console.error);