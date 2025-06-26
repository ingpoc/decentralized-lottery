const { Connection, PublicKey } = require('@solana/web3.js');
const { struct, u8, publicKey } = require('@solana/buffer-layout');

async function checkGlobalConfigAdmin() {
  const connection = new Connection('https://api.devnet.solana.com');
  const programId = new PublicKey('9SL8XkX3pvqZ2fjiLMhCFfQn7Gfmpd9ru8rtHFsAPVgq');
  
  // Calculate global config PDA
  const [globalConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from('global_config')], 
    programId
  );
  
  console.log('Global Config PDA:', globalConfigPda.toString());
  
  try {
    const accountInfo = await connection.getAccountInfo(globalConfigPda);
    if (!accountInfo) {
      console.log('Global config account does not exist');
      return;
    }
    
    console.log('Account data length:', accountInfo.data.length);
    console.log('Raw data:', accountInfo.data.toString('hex'));
    
    // Skip the 8-byte discriminator and read the admin pubkey
    const adminBytes = accountInfo.data.slice(8, 40); // Next 32 bytes after discriminator
    const adminPubkey = new PublicKey(adminBytes);
    
    console.log('Admin in global config:', adminPubkey.toString());
    console.log('Current wallet:      ', '7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4');
    console.log('Admin matches current wallet:', adminPubkey.toString() === '7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4');
    
  } catch (error) {
    console.error('Error:', error);
  }
}

checkGlobalConfigAdmin();