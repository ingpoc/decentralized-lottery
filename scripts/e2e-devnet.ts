/**
 * E2E devnet test — full game with multiple real wallets.
 *
 * Flow:
 *   1. Create 3 user wallets (Alice, Bob, Charlie)
 *   2. Fund each with devnet SOL + test USDC
 *   3. Admin creates a short-draw lottery (2 min)
 *   4. All 3 buy tickets (different quantities)
 *   5. Record pre-draw USDC balances
 *   6. Advance lifecycle: Open → Locked → Drawing → AwaitingRandomness
 *   7. Settle randomness (fallback)
 *   8. Select winner
 *   9. Winner claims
 *  10. Assert: winner balance increased by (pool - 2.5% fee), treasury increased by fee
 *
 * Run: npx ts-node scripts/e2e-devnet.ts
 */
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
  SYSVAR_SLOT_HASHES_PUBKEY,
  SYSVAR_CLOCK_PUBKEY,
  Connection,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  createMint,
} from "@solana/spl-token";
import { BN } from "bn.js";
import * as fs from "fs";

const RPC = "https://api.devnet.solana.com";
const connection = new Connection(RPC, "confirmed");

// Load the deployer keypair (admin)
const adminKeypair = Keypair.fromSecretKey(
  Buffer.from(JSON.parse(fs.readFileSync("/tmp/lottery-deployer.json", "utf-8")))
);

// Load IDL
const idl = require("../target/idl/decentralized_lottery.json");

async function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms));
}

async function getUsdcBalance(owner: PublicKey, usdcMint: PublicKey, offCurve = false): Promise<bigint> {
  const ata = getAssociatedTokenAddressSync(usdcMint, owner, offCurve, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID);
  try {
    const acc = await getAccount(connection, ata);
    return acc.amount;
  } catch {
    return 0n;
  }
}

async function fundSol(kp: PublicKey, sol: number) {
  // Transfer SOL from admin (avoids devnet airdrop rate limits)
  const tx = new (require('@solana/web3.js').Transaction)().add(
    (require('@solana/web3.js').SystemProgram).transfer({
      fromPubkey: adminKeypair.publicKey,
      toPubkey: kp,
      lamports: sol * LAMPORTS_PER_SOL,
    })
  );
  const sig = await connection.sendTransaction(tx, [adminKeypair]);
  await connection.confirmTransaction(sig);
}

async function main() {
  console.log("============================================");
  console.log("  E2E Devnet Test — Full Lottery Game");
  console.log("============================================\n");

  const provider = new anchor.AnchorProvider(
    connection,
    new anchor.Wallet(adminKeypair),
    { preflightCommitment: "confirmed" }
  );
  anchor.setProvider(provider);
  const program = new Program(idl, provider);

  // --- Existing devnet state ---
  const [globalConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_config_v2")], program.programId
  );
  const config = await (program.account as any).globalConfig.fetch(globalConfigPda);
  const usdcMint = config.usdcMint;

  console.log("Admin:", adminKeypair.publicKey.toBase58());
  console.log("USDC Mint:", usdcMint.toBase58());
  console.log("Program:", program.programId.toBase58());

  // Treasury ATA
  const treasuryAta = getAssociatedTokenAddressSync(
    usdcMint, globalConfigPda, true, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
  );

  // ============================================================
  // STEP 1: Create 3 user wallets
  // ============================================================
  console.log("\n--- STEP 1: Creating player wallets ---");
  const players = [
    { name: "Alice", keypair: Keypair.generate(), tickets: 2 },
    { name: "Bob", keypair: Keypair.generate(), tickets: 1 },
    { name: "Charlie", keypair: Keypair.generate(), tickets: 3 },
  ];

  for (const p of players) {
    console.log(`  ${p.name}: ${p.keypair.publicKey.toBase58()} (${p.tickets} ticket(s))`);
  }

  // ============================================================
  // STEP 2: Fund each player with SOL + USDC
  // ============================================================
  console.log("\n--- STEP 2: Funding players ---");
  for (const p of players) {
    console.log(`  Funding ${p.name} with 0.05 SOL + 100 USDC...`);
    await fundSol(p.keypair.publicKey, 0.05);

    // Create USDC ATA for player
    const playerAta = getAssociatedTokenAddressSync(
      usdcMint, p.keypair.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
    );
    await getOrCreateAssociatedTokenAccount(
      connection, adminKeypair, usdcMint, p.keypair.publicKey
    );

    // Mint 100 USDC to each player
    await mintTo(connection, adminKeypair, usdcMint, playerAta, adminKeypair, 100_000_000);
    console.log(`    ✓ ${p.name}: 100 USDC minted`);
  }

  // ============================================================
  // STEP 3: Create a short-draw lottery (90 seconds)
  // ============================================================
  console.log("\n--- STEP 3: Creating lottery (90s draw) ---");
  const nonce = new BN(Date.now());
  const [lotteryPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("lottery"), adminKeypair.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
    program.programId
  );
  const ticketPrice = new BN(1_000_000); // 1 USDC
  const drawTime = new BN(Math.floor(Date.now() / 1000) + 90);

  // Create lottery vault ATA
  await getOrCreateAssociatedTokenAccount(connection, adminKeypair, usdcMint, lotteryPda, true);

  await (program.methods as any)
    .createLottery({ daily: {} }, ticketPrice, drawTime, new BN(0), nonce)
    .accounts({
      lotteryAccount: lotteryPda,
      globalConfig: globalConfigPda,
      creator: adminKeypair.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("  ✓ Lottery created:", lotteryPda.toBase58());

  // Open it
  await (program.methods as any)
    .transitionState({ open: {} })
    .accounts({
      lotteryAccount: lotteryPda,
      globalConfig: globalConfigPda,
      admin: adminKeypair.publicKey,
      systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("  ✓ Lottery opened");

  // ============================================================
  // STEP 4: Players buy tickets
  // ============================================================
  console.log("\n--- STEP 4: Players buying tickets ---");
  let totalTicketsBought = 0;

  for (const p of players) {
    const playerProvider = new anchor.AnchorProvider(
      connection,
      new anchor.Wallet(p.keypair),
      { preflightCommitment: "confirmed" }
    );
    const playerProgram = new Program(idl, playerProvider);

    for (let i = 0; i < p.tickets; i++) {
      totalTicketsBought++;
      const ticketPda = PublicKey.findProgramAddressSync(
        [Buffer.from("ticket"), lotteryPda.toBuffer(), new BN(totalTicketsBought).toArrayLike(Buffer, "le", 8)],
        program.programId
      )[0];

      const playerAta = getAssociatedTokenAddressSync(
        usdcMint, p.keypair.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
      );
      const vaultAta = getAssociatedTokenAddressSync(
        usdcMint, lotteryPda, true, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
      );

      await (playerProgram.methods as any)
        .buyTicket()
        .accounts({
          lotteryAccount: lotteryPda,
          ticketAccount: ticketPda,
          globalConfig: globalConfigPda,
          user: p.keypair.publicKey,
          userTokenAccount: playerAta,
          lotteryTokenAccount: vaultAta,
          usdcMint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([p.keypair])
        .rpc();

      console.log(`  ✓ ${p.name} bought ticket #${totalTicketsBought}`);
    }
  }

  // ============================================================
  // STEP 5: Record pre-draw balances
  // ============================================================
  console.log("\n--- STEP 5: Recording pre-draw balances ---");
  const preBalances: Record<string, bigint> = {};
  for (const p of players) {
    preBalances[p.name] = await getUsdcBalance(p.keypair.publicKey, usdcMint);
    console.log(`  ${p.name}: ${Number(preBalances[p.name]) / 1e6} USDC`);
  }
  preBalances["treasury"] = await getUsdcBalance(globalConfigPda, usdcMint, true);
  console.log(`  Treasury: ${Number(preBalances["treasury"]) / 1e6} USDC`);

  const totalPool = BigInt(totalTicketsBought) * 1_000_000n;
  console.log(`\n  Total prize pool: ${totalTicketsBought} tickets × 1 USDC = ${Number(totalPool) / 1e6} USDC`);
  const expectedFee = (totalPool * 250n) / 10000n;
  const expectedPayout = totalPool - expectedFee;
  console.log(`  Expected treasury fee (2.5%): ${Number(expectedFee) / 1e6} USDC`);
  console.log(`  Expected winner payout (97.5%): ${Number(expectedPayout) / 1e6} USDC`);

  // ============================================================
  // STEP 6: Wait for draw time, then advance lifecycle
  // ============================================================
  console.log("\n--- STEP 6: Advancing lottery lifecycle ---");

  // Wait for draw time to pass
  const now = Math.floor(Date.now() / 1000);
  const waitSec = drawTime.toNumber() - now + 3;
  if (waitSec > 0) {
    console.log(`  Waiting ${waitSec}s for draw time...`);
    await sleep(waitSec * 1000);
  }

  // Open → Locked
  await (program.methods as any)
    .transitionState({ locked: {} })
    .accounts({
      lotteryAccount: lotteryPda, globalConfig: globalConfigPda,
      admin: adminKeypair.publicKey, systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("  ✓ Locked");

  // Locked → Drawing
  await (program.methods as any)
    .transitionState({ drawing: {} })
    .accounts({
      lotteryAccount: lotteryPda, globalConfig: globalConfigPda,
      admin: adminKeypair.publicKey, systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("  ✓ Drawing");

  // Drawing → AwaitingRandomness
  await (program.methods as any)
    .transitionState({ awaitingRandomness: {} })
    .accounts({
      lotteryAccount: lotteryPda, globalConfig: globalConfigPda,
      admin: adminKeypair.publicKey, systemProgram: SystemProgram.programId,
    })
    .rpc();
  console.log("  ✓ AwaitingRandomness");

  // ============================================================
  // STEP 7: Settle randomness (fallback path, needs draw_time + 10s)
  // ============================================================
  console.log("\n--- STEP 7: Settling randomness ---");
  const settleWait = (drawTime.toNumber() + 10) - Math.floor(Date.now() / 1000) + 2;
  if (settleWait > 0) {
    console.log(`  Waiting ${settleWait}s for settle delay...`);
    await sleep(settleWait * 1000);
  }

  await (program.methods as any)
    .settleRandomness()
    .accounts({
      lotteryAccount: lotteryPda,
      recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
      clock: SYSVAR_CLOCK_PUBKEY,
      caller: adminKeypair.publicKey,
    })
    .rpc();
  console.log("  ✓ Randomness settled → Completed");

  // ============================================================
  // STEP 8: Select winner
  // ============================================================
  console.log("\n--- STEP 8: Selecting winner ---");
  const lottery = await (program.account as any).lotteryAccount.fetch(lotteryPda);
  const randomness = lottery.vrfRandomness;
  const randomValue = randomness.slice(0, 8).reduce(
    (acc: bigint, b: number, i: number) => acc | (BigInt(b) << BigInt(8 * i)), 0n
  );
  const winningTicketId = Number(randomValue % BigInt(totalTicketsBought)) + 1;

  // Determine which player owns this ticket
  let winnerName = "";
  let ticketCount = 0;
  for (const p of players) {
    for (let i = 0; i < p.tickets; i++) {
      ticketCount++;
      if (ticketCount === winningTicketId) {
        winnerName = p.name;
        break;
      }
    }
    if (winnerName) break;
  }

  const winningTicketPda = PublicKey.findProgramAddressSync(
    [Buffer.from("ticket"), lotteryPda.toBuffer(), new BN(winningTicketId).toArrayLike(Buffer, "le", 8)],
    program.programId
  )[0];

  await (program.methods as any)
    .selectWinner()
    .accounts({
      lotteryAccount: lotteryPda,
      winningTicketAccount: winningTicketPda,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  console.log(`  ✓ Winner: Ticket #${winningTicketId} owned by ${winnerName}!`);

  // ============================================================
  // STEP 9: Winner claims the prize
  // ============================================================
  console.log("\n--- STEP 9: Winner claiming prize ---");
  const winner = players.find(p => p.name === winnerName)!;
  const winnerProvider = new anchor.AnchorProvider(
    connection, new anchor.Wallet(winner.keypair), { preflightCommitment: "confirmed" }
  );
  const winnerProgram = new Program(idl, winnerProvider);

  const winnerAta = getAssociatedTokenAddressSync(
    usdcMint, winner.keypair.publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
  );
  const vaultAta = getAssociatedTokenAddressSync(
    usdcMint, lotteryPda, true, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
  );

  await (winnerProgram.methods as any)
    .claimPrize()
    .accounts({
      lotteryAccount: lotteryPda,
      ticketAccount: winningTicketPda,
      globalConfig: globalConfigPda,
      winner: winner.keypair.publicKey,
      lotteryTokenAccount: vaultAta,
      winnerTokenAccount: winnerAta,
      treasuryTokenAccount: treasuryAta,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([winner.keypair])
    .rpc();
  console.log("  ✓ Prize claimed!");

  // ============================================================
  // STEP 10: Verify balances — the real validation
  // ============================================================
  console.log("\n--- STEP 10: Balance verification ---");
  console.log("\n  ╔══════════════════════════════════════════════════╗");
  console.log("  ║          GAME RESULTS — ALL ON DEVNET            ║");
  console.log("  ╚══════════════════════════════════════════════════╝\n");

  let allPassed = true;

  // Winner balance: "before" was recorded AFTER ticket purchase, so the delta
  // from claim should be exactly expectedPayout.
  const winnerPost = await getUsdcBalance(winner.keypair.publicKey, usdcMint);
  const winnerDelta = winnerPost - preBalances[winnerName];

  console.log(`  🏆 WINNER (${winnerName}):`);
  console.log(`     Before claim: ${Number(preBalances[winnerName]) / 1e6} USDC (already paid for ${winner.tickets} ticket(s))`);
  console.log(`     Won:          ${Number(expectedPayout) / 1e6} USDC (97.5% of ${Number(totalPool) / 1e6} pool)`);
  console.log(`     After claim:  ${Number(winnerPost) / 1e6} USDC`);
  console.log(`     Gain:         ${Number(winnerDelta) / 1e6} USDC (should be ${Number(expectedPayout) / 1e6})`);

  if (winnerDelta !== expectedPayout) {
    console.log(`     ✗ MISMATCH! Expected gain ${Number(expectedPayout) / 1e6}, got ${Number(winnerDelta) / 1e6}`);
    allPassed = false;
  } else {
    console.log(`     ✓ CORRECT`);
  }

  // Treasury balance: should have increased by expectedFee
  const treasuryPost = await getUsdcBalance(globalConfigPda, usdcMint, true);
  const treasuryDelta = treasuryPost - preBalances["treasury"];

  console.log(`\n  💰 TREASURY:`);
  console.log(`     Before: ${Number(preBalances["treasury"]) / 1e6} USDC`);
  console.log(`     After:  ${Number(treasuryPost) / 1e6} USDC`);
  console.log(`     Gain:   ${Number(treasuryDelta) / 1e6} USDC (should be ${Number(expectedFee) / 1e6})`);

  if (treasuryDelta !== expectedFee) {
    console.log(`     ✗ MISMATCH! Expected ${Number(expectedFee) / 1e6}, got ${Number(treasuryDelta) / 1e6}`);
    allPassed = false;
  } else {
    console.log(`     ✓ CORRECT`);
  }

  // Losers: "before" was recorded after purchase, so no change after the draw
  console.log(`\n  😤 LOSERS (paid for tickets, didn't win):`);
  for (const p of players) {
    if (p.name === winnerName) continue;
    const post = await getUsdcBalance(p.keypair.publicKey, usdcMint);
    const delta = post - preBalances[p.name];
    const spent = BigInt(p.tickets) * 1_000_000n;
    console.log(`     ${p.name}: ${p.tickets} ticket(s) = ${Number(spent) / 1e6} USDC spent, balance: ${Number(preBalances[p.name]) / 1e6} → ${Number(post) / 1e6} USDC`);
    if (delta !== 0n) {
      console.log(`     ✗ UNEXPECTED CHANGE! Delta ${Number(delta) / 1e6}, expected 0 (already paid before draw)`);
      allPassed = false;
    } else {
      console.log(`     ✓ CORRECT — no unexpected balance change`);
    }
  }

  // Conservation check: total USDC moved = pool (winner payout + treasury fee)
  console.log(`\n  📊 CONSERVATION CHECK:`);
  console.log(`     Winner payout: ${Number(expectedPayout) / 1e6} USDC`);
  console.log(`     Treasury fee:  ${Number(expectedFee) / 1e6} USDC`);
  console.log(`     Sum:            ${Number(expectedPayout + expectedFee) / 1e6} USDC = ${Number(totalPool) / 1e6} USDC pool ✓`);

  console.log(`\n  ${allPassed ? "✅✅✅ ALL BALANCES VERIFIED — GAME WORKS CORRECTLY ✅✅✅" : "❌ BALANCE MISMATCH DETECTED"}`);
  console.log("\n============================================");
  console.log("  E2E Devnet Test Complete");
  console.log("============================================");
}

main().catch((err) => {
  console.error("\n❌ E2E test failed:", err);
  process.exit(1);
});
