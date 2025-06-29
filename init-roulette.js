const anchor = require('@coral-xyz/anchor');
const { PublicKey, SystemProgram } = require('@solana/web3.js');
const { getAssociatedTokenAddress } = require('@solana/spl-token');

async function initializeRoulette() {
  // Set up provider
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  // Load the roulette program
  const idl = require('./target/idl/decentralized_roulette.json');
  const program = new anchor.Program(idl, provider);

  console.log('Program ID:', program.programId.toString());
  console.log('Authority:', provider.wallet.publicKey.toString());

  // USDC mint on devnet
  const usdcMint = new PublicKey('Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr');
  
  // Global config PDA
  const [globalConfigPDA] = PublicKey.findProgramAddressSync(
    [Buffer.from('global_config_v2')],
    program.programId
  );

  console.log('Global Config PDA:', globalConfigPDA.toString());

  // Treasury token account (associated with authority for now)
  const treasuryTokenAccount = await getAssociatedTokenAddress(
    usdcMint,
    provider.wallet.publicKey
  );

  console.log('Treasury Token Account:', treasuryTokenAccount.toString());

  try {
    // Check if already initialized
    try {
      const globalConfig = await program.account.globalConfig.fetch(globalConfigPDA);
      console.log('Roulette program already initialized!');
      console.log('Global Config:', globalConfig);
      return;
    } catch (error) {
      console.log('Global config not found, proceeding with initialization...');
    }

    // Initialize the roulette program
    console.log('Initializing roulette program...');
    const tx = await program.methods
      .initialize()
      .accounts({
        globalConfig: globalConfigPDA,
        authority: provider.wallet.publicKey,
        usdcMint: usdcMint,
        treasuryTokenAccount: treasuryTokenAccount,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log('✅ Roulette program initialized successfully!');
    console.log('Transaction signature:', tx);

    // Fetch and display the initialized config
    const globalConfig = await program.account.globalConfig.fetch(globalConfigPDA);
    console.log('Initialized Global Config:', globalConfig);

  } catch (error) {
    console.error('❌ Error initializing roulette program:', error);
    if (error.logs) {
      console.log('Program logs:', error.logs);
    }
  }
}

initializeRoulette().catch(console.error);