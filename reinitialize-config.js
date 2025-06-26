const anchor = require('@coral-xyz/anchor');
const { Connection, PublicKey, Keypair } = require('@solana/web3.js');

async function reinitializeGlobalConfig() {
  // Set up connection and provider
  const connection = new Connection('https://api.devnet.solana.com');
  const programId = new PublicKey('9SL8XkX3pvqZ2fjiLMhCFfQn7Gfmpd9ru8rtHFsAPVgq');
  
  console.log('Current wallet:', '7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4');
  
  // Calculate global config PDA  
  const [globalConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('global_config')], 
    programId
  );
  
  console.log('Global Config PDA:', globalConfigPda.toString());
  
  // Check if account exists
  const existingAccount = await connection.getAccountInfo(globalConfigPda);
  if (existingAccount) {
    console.log('Global config already exists with admin:', existingAccount.data.slice(8, 40).toString('hex'));
    console.log('Account lamports:', existingAccount.lamports);
    console.log('Account owner:', existingAccount.owner.toString());
    
    // Try to close the account by transferring all lamports
    console.log('\\nThis account was created with a different admin wallet.');
    console.log('You need to either:');
    console.log('1. Use the original admin wallet (5P6sD6ra5Y4fGC2fm4RntE3i1j1THfXjoXoTPVd2y9jZ)');
    console.log('2. Deploy a new version of the program with a different global config seed');
    console.log('3. Add an admin update instruction to your program');
  } else {
    console.log('Global config does not exist - can initialize');
  }
}

reinitializeGlobalConfig().catch(console.error);