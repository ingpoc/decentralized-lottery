const anchor = require('@coral-xyz/anchor');
const { Connection, PublicKey, Keypair } = require('@solana/web3.js');
const fs = require('fs');
const path = require('path');

// Load the IDL
const idlPath = path.join(__dirname, 'target/idl/decentralized_roulette.json');
const idl = JSON.parse(fs.readFileSync(idlPath, 'utf8'));

// Program ID
const PROGRAM_ID = new PublicKey('4ZVg5wU59Tr6pKAfxkTFsF2cffGrVRM2xqt1WbPUJrUB');

// USDC mint (devnet)
const USDC_MINT_DEVNET = new PublicKey('Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr');

async function createFirstGame() {
    try {
        // Set up connection and wallet
        const connection = new Connection('https://api.devnet.solana.com', 'confirmed');
        
        // Load wallet from default location
        const walletPath = process.env.HOME + '/.config/solana/id.json';
        const walletKeypair = Keypair.fromSecretKey(
            Buffer.from(JSON.parse(fs.readFileSync(walletPath, 'utf8')))
        );
        
        console.log('Using wallet:', walletKeypair.publicKey.toBase58());
        
        // Set up Anchor
        const wallet = new anchor.Wallet(walletKeypair);
        const provider = new anchor.AnchorProvider(connection, wallet, {});
        const program = new anchor.Program(idl, PROGRAM_ID, provider);
        
        // Derive PDAs
        const [globalConfigPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from('global_config_v2')],
            PROGRAM_ID
        );
        
        const nonce = Date.now();
        const [newRoulettePDA] = PublicKey.findProgramAddressSync(
            [
                Buffer.from('roulette'),
                walletKeypair.publicKey.toBuffer(),
                Buffer.from(nonce.toString().padStart(8, '0'), 'utf8').slice(0, 8)
            ],
            PROGRAM_ID
        );
        
        const [rouletteTokenPDA] = PublicKey.findProgramAddressSync(
            [Buffer.from('roulette_token'), newRoulettePDA.toBuffer()],
            PROGRAM_ID
        );
        
        console.log('Creating first roulette game...');
        console.log('Global config PDA:', globalConfigPDA.toBase58());
        console.log('New roulette PDA:', newRoulettePDA.toBase58());
        console.log('Roulette token PDA:', rouletteTokenPDA.toBase58());
        console.log('Nonce:', nonce);
        
        // Create the first game
        const tx = await program.methods
            .createNextGame(new anchor.BN(nonce))
            .accounts({
                newRoulette: newRoulettePDA,
                globalConfig: globalConfigPDA,
                caller: walletKeypair.publicKey,
                usdcMint: USDC_MINT_DEVNET,
                rouletteTokenAccount: rouletteTokenPDA,
                tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
                systemProgram: anchor.web3.SystemProgram.programId,
                rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            })
            .rpc();
        
        console.log('✅ First roulette game created successfully!');
        console.log('Transaction signature:', tx);
        console.log('Roulette game address:', newRoulettePDA.toBase58());
        
        // Verify the game was created
        const rouletteAccount = await program.account.rouletteAccount.fetch(newRoulettePDA);
        console.log('Game state:', rouletteAccount.state);
        console.log('Betting end time:', new Date(rouletteAccount.bettingEndTime.toNumber() * 1000));
        console.log('End time:', new Date(rouletteAccount.endTime.toNumber() * 1000));
        
    } catch (error) {
        console.error('Error creating first game:', error);
        
        if (error.message.includes('already in use')) {
            console.log('Game might already exist. Trying different nonce...');
            // Could retry with different nonce here
        }
    }
}

createFirstGame();