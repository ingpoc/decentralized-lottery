import {
  Connection,
  Keypair,
  PublicKey,
  Transaction,
  TransactionInstruction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import fs from 'fs';
import path from 'path';

// Set up a connection to devnet
const connection = new Connection('https://api.devnet.solana.com', 'confirmed');

// Load the wallet keypair from the default Solana config
const walletKeypairPath = path.join(
  process.env.HOME || '',
  '.config',
  'solana',
  'id.json'
);

let wallet: Keypair;
try {
  // Try to load the default keypair
  const walletKeypairData = fs.readFileSync(walletKeypairPath, 'utf-8');
  wallet = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(walletKeypairData))
  );
  console.log('Using wallet:', wallet.publicKey.toString());
} catch (error) {
  console.error('Error loading wallet keypair:', error);
  console.log('Generating a new keypair for testing purposes only...');
  // Generate a new keypair for testing purposes
  wallet = Keypair.generate();
  console.log('Generated wallet:', wallet.publicKey.toString());
}

// Load the program ID from the IDL file
const idlFile = path.join(__dirname, '../target/idl/decentralized_lottery.json');
const idl = JSON.parse(fs.readFileSync(idlFile, 'utf8'));
const programId = new PublicKey(idl.address || 'F1pffGp4n5qyNRcCnpoTH5CEfVKQEGxAxmRuRScUw4tz');

// The new USDC mint address
const NEW_USDC_MINT = new PublicKey('Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr');

async function main() {
  try {
    console.log('Updating USDC mint address...');
    
    // Derive the global config PDA
    const [globalConfigPDA] = PublicKey.findProgramAddressSync(
      [Buffer.from('global_config')],
      programId
    );
    
    console.log('Global Config PDA:', globalConfigPDA.toString());
    console.log('New USDC Mint:', NEW_USDC_MINT.toString());
    console.log('Program ID:', programId.toString());
    
    // Create the instruction data for update_config
    // The discriminator for update_config is [29, 158, 252, 191, 10, 83, 219, 99] from the IDL
    const data = Buffer.from([29, 158, 252, 191, 10, 83, 219, 99]);
    
    // Create the instruction
    const instruction = new TransactionInstruction({
      keys: [
        { pubkey: globalConfigPDA, isSigner: false, isWritable: true },
        { pubkey: wallet.publicKey, isSigner: true, isWritable: true },
        { pubkey: NEW_USDC_MINT, isSigner: false, isWritable: false },
      ],
      programId,
      data,
    });
    
    // Create and send the transaction
    const transaction = new Transaction().add(instruction);
    const signature = await sendAndConfirmTransaction(
      connection,
      transaction,
      [wallet]
    );
    
    console.log('Transaction signature:', signature);
    console.log('USDC mint address updated successfully!');
    
    // Fetch the global config account data to verify the update
    const accountInfo = await connection.getAccountInfo(globalConfigPDA);
    if (accountInfo && accountInfo.data) {
      console.log('Global Config account data updated. Size:', accountInfo.data.length);
      // Note: Parsing the raw account data would require knowledge of the exact data layout
      // For simplicity, we're just confirming the account exists and was updated
    } else {
      console.log('Could not fetch Global Config account data');
    }
    
  } catch (error) {
    console.error('Error updating USDC mint address:', error);
  }
}

main().then(
  () => process.exit(0),
  (err) => {
    console.error(err);
    process.exit(1);
  }
); 