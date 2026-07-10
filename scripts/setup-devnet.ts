/**
 * Devnet setup script — initializes the global config and creates the first lottery.
 *
 * Prerequisites:
 *   - Program deployed to devnet (anchor deploy --provider.cluster devnet)
 *   - Devnet USDC mint created (or use an existing one)
 *   - Admin wallet funded with devnet SOL + USDC
 *
 * Usage:
 *   ts-node scripts/setup-devnet.ts
 *
 * Environment variables (or hardcoded defaults below):
 *   DEVNET_USDC_MINT  — USDC mint address on devnet
 */
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DecentralizedLottery } from "../target/types/decentralized_lottery";
import {
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  createMint,
  mintTo,
} from "@solana/spl-token";
import { BN } from "bn.js";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.DecentralizedLottery as Program<DecentralizedLottery>;
  const admin = (provider.wallet as anchor.Wallet).payer;
  const connection = provider.connection;

  console.log("=== Devnet Setup ===");
  console.log("Admin:", admin.publicKey.toBase58());
  console.log("Program:", program.programId.toBase58());
  console.log("Balance:", (await connection.getBalance(admin.publicKey)) / LAMPORTS_PER_SOL, "SOL");

  // --- 1. Derive global config PDA ---
  const [globalConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_config_v2")],
    program.programId
  );
  console.log("\nGlobalConfig PDA:", globalConfigPda.toBase58());

  // --- 2. Create or use a USDC mint ---
  // On devnet, create our own test USDC mint (6 decimals) for the lottery.
  // In production this would be the real USDC mint.
  let usdcMint: PublicKey;
  const existingMint = process.env.DEVNET_USDC_MINT;
  if (existingMint) {
    usdcMint = new PublicKey(existingMint);
    console.log("Using existing USDC mint:", usdcMint.toBase58());
  } else {
    console.log("\nCreating test USDC mint (6 decimals)...");
    usdcMint = await createMint(connection, admin, admin.publicKey, null, 6);
    console.log("USDC mint:", usdcMint.toBase58());
  }

  // --- 3. Create treasury ATA (owned by globalConfig PDA) ---
  const treasuryAta = getAssociatedTokenAddressSync(
    usdcMint, globalConfigPda, true, undefined, undefined, ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID
  );
  console.log("Treasury ATA:", treasuryAta.toBase58());
  await getOrCreateAssociatedTokenAccount(connection, admin, usdcMint, globalConfigPda, true);

  // --- 4. Initialize global config ---
  try {
    const configInfo = await program.account.globalConfig.fetchNullable(globalConfigPda);
    if (configInfo) {
      console.log("\nGlobalConfig already initialized — skipping");
    } else {
      throw new Error("not initialized");
    }
  } catch {
    console.log("\nInitializing global config...");
    await program.methods
      .initialize()
      .accounts({
        globalConfig: globalConfigPda,
        admin: admin.publicKey,
        usdcMint,
        treasuryTokenAccount: treasuryAta,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("✓ Global config initialized");
  }

  // --- 5. Fund admin with USDC (if we created the mint) ---
  if (!existingMint) {
    const adminAta = getAssociatedTokenAddressSync(
      usdcMint, admin.publicKey, false, undefined, undefined, ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID
    );
    await getOrCreateAssociatedTokenAccount(connection, admin, usdcMint, admin.publicKey);
    await mintTo(connection, admin, usdcMint, adminAta, admin, 10_000_000_000); // 10,000 USDC
    console.log("✓ Minted 10,000 test USDC to admin");
  }

  // --- 6. Create the first lottery ---
  const nonce = new BN(Date.now());
  const [lotteryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("lottery"), admin.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
    program.programId
  );

  const ticketPrice = new BN(1_000_000); // 1 USDC
  const drawTime = new BN(Math.floor(Date.now() / 1000) + 7 * 24 * 60 * 60); // 7 days

  // Create lottery vault ATA
  const lotteryVault = getAssociatedTokenAddressSync(
    usdcMint, lotteryPda, true, undefined, undefined, ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID
  );
  await getOrCreateAssociatedTokenAccount(connection, admin, usdcMint, lotteryPda, true);

  console.log("\nCreating daily lottery...");
  console.log("  Lottery PDA:", lotteryPda.toBase58());
  console.log("  Ticket price: 1 USDC");
  console.log("  Draw time:", new Date(drawTime.toNumber() * 1000).toISOString());

  await program.methods
    .createLottery({ daily: {} }, ticketPrice, drawTime, new BN(0), nonce)
    .accounts({
      lotteryAccount: lotteryPda,
      globalConfig: globalConfigPda,
      creator: admin.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("✓ Lottery created");

  // --- 7. Open the lottery ---
  await program.methods
    .transitionState({ open: {} })
    .accounts({
      lotteryAccount: lotteryPda,
      globalConfig: globalConfigPda,
      admin: admin.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("✓ Lottery opened for ticket purchases");

  console.log("\n=== Setup Complete ===");
  console.log("Program ID:", program.programId.toBase58());
  console.log("USDC Mint:", usdcMint.toBase58());
  console.log("GlobalConfig:", globalConfigPda.toBase58());
  console.log("Active Lottery:", lotteryPda.toBase58());
  console.log("\nFrontend .env.local values:");
  console.log(`NEXT_PUBLIC_PROGRAM_ID=${program.programId.toBase58()}`);
  console.log(`NEXT_PUBLIC_USDC_MINT=${usdcMint.toBase58()}`);
  console.log(`NEXT_PUBLIC_RPC_URL=https://api.devnet.solana.com`);
}

main().catch((err) => {
  console.error("Setup failed:", err);
  process.exit(1);
});
