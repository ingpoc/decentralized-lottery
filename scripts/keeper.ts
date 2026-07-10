/**
 * Lottery Keeper — automates the draw lifecycle for active lotteries.
 *
 * Polls for lotteries that have passed their draw_time and advances them:
 *   Open → Locked → Drawing → AwaitingRandomness → (settle_randomness) → Completed
 *
 * Run: npm run keeper
 *
 * Requires ANCHOR_PROVIDER_URL set to the target cluster (devnet/localnet).
 */
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DecentralizedLottery } from "../target/types/decentralized_lottery";
import {
  PublicKey,
  SystemProgram,
  SYSVAR_SLOT_HASHES_PUBKEY,
  SYSVAR_CLOCK_PUBKEY,
} from "@solana/web3.js";
import { BN } from "bn.js";

const POLL_INTERVAL_MS = 15_000; // 15 seconds

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.DecentralizedLottery as Program<DecentralizedLottery>;
  const admin = (provider.wallet as anchor.Wallet).payer;

  console.log("=== Lottery Keeper ===");
  console.log("Admin:", admin.publicKey.toBase58());
  console.log("Program:", program.programId.toBase58());
  console.log(`Polling every ${POLL_INTERVAL_MS / 1000}s...\n`);

  const [globalConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_config_v2")],
    program.programId
  );

  // Verify admin matches config
  try {
    const config = await (program.account as any).globalConfig.fetch(globalConfigPda);
    if (config.admin.toBase58() !== admin.publicKey.toBase58()) {
      console.error("ERROR: Connected wallet is not the admin in GlobalConfig.");
      console.error(`Config admin: ${config.admin.toBase58()}`);
      process.exit(1);
    }
    console.log("✓ Admin verified\n");
  } catch {
    console.error("GlobalConfig not found. Run setup:devnet first.");
    process.exit(1);
  }

  async function tick() {
    const now = Math.floor(Date.now() / 1000);
    let allLotteries: any[];

    try {
      allLotteries = await (program.account as any).lotteryAccount.all();
    } catch (err) {
      console.error("Failed to fetch lotteries:", err);
      return;
    }

    for (const { publicKey, account } of allLotteries) {
      const state = Object.keys(account.state)[0];
      const drawTime = account.drawTime;
      const lotteryLabel = publicKey.toBase58().slice(0, 8);

      // Only process lotteries past their draw time
      if (now < drawTime && state !== "awaitingRandomness") continue;

      try {
        if (state === "open") {
          console.log(`[${lotteryLabel}] Open → Locked`);
          await program.methods
            .transitionState({ locked: {} })
            .accounts({
              lotteryAccount: publicKey,
              globalConfig: globalConfigPda,
              admin: admin.publicKey,
              systemProgram: SystemProgram.programId,
            })
            .rpc();

        } else if (state === "locked") {
          console.log(`[${lotteryLabel}] Locked → Drawing`);
          await program.methods
            .transitionState({ drawing: {} })
            .accounts({
              lotteryAccount: publicKey,
              globalConfig: globalConfigPda,
              admin: admin.publicKey,
              systemProgram: SystemProgram.programId,
            })
            .rpc();

        } else if (state === "drawing") {
          console.log(`[${lotteryLabel}] Drawing → AwaitingRandomness`);
          await program.methods
            .transitionState({ awaitingRandomness: {} })
            .accounts({
              lotteryAccount: publicKey,
              globalConfig: globalConfigPda,
              admin: admin.publicKey,
              systemProgram: SystemProgram.programId,
            })
            .rpc();

        } else if (state === "awaitingRandomness") {
          // settle_randomness requires draw_time + 10 seconds
          if (now < drawTime + 10) {
            const wait = drawTime + 10 - now;
            console.log(`[${lotteryLabel}] AwaitingRandomness — waiting ${wait}s for settle delay`);
            continue;
          }

          console.log(`[${lotteryLabel}] Settling randomness (fallback) → Completed`);
          await program.methods
            .settleRandomness()
            .accounts({
              lotteryAccount: publicKey,
              recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
              clock: SYSVAR_CLOCK_PUBKEY,
              caller: admin.publicKey,
            })
            .rpc();

          // Now select the winner
          // Need to derive the winning ticket — but we don't know which ticket ID won
          // until we read the randomness. Fetch the lottery to compute it.
          const lottery = await (program.account as any).lotteryAccount.fetch(publicKey);
          const randomness = lottery.vrfRandomness;
          if (!randomness) {
            console.log(`[${lotteryLabel}] No randomness yet, skipping winner selection`);
            continue;
          }

          // Compute winning ticket ID (same logic as the program)
          const randomValue = randomness.slice(0, 8).reduce(
            (acc: bigint, b: number, i: number) => acc | (BigInt(b) << BigInt(8 * i)),
            0n
          );
          const winningId = Number(randomValue % BigInt(lottery.totalTickets)) + 1;

          // Derive winning ticket PDA
          const ticketPda = PublicKey.findProgramAddressSync(
            [Buffer.from("ticket"), publicKey.toBuffer(), new BN(winningId).toArrayLike(Buffer, "le", 8)],
            program.programId
          )[0];

          console.log(`[${lotteryLabel}] Selecting winner: ticket #${winningId}`);
          await program.methods
            .selectWinner()
            .accounts({
              lotteryAccount: publicKey,
              winningTicketAccount: ticketPda,
              systemProgram: SystemProgram.programId,
            })
            .rpc();

          console.log(`[${lotteryLabel}] ✓ Draw complete — winner selected`);
        }
      } catch (err: any) {
        // Log but don't crash — keeper should be resilient
        const msg = err?.message || String(err);
        if (msg.includes("0x1")) {
          // Insufficient funds — stop
          console.error(`[${lotteryLabel}] Insufficient funds. Top up admin wallet.`);
          process.exit(1);
        }
        console.error(`[${lotteryLabel}] Error in state ${state}:`, msg.slice(0, 120));
      }
    }
  }

  // Run immediately, then on interval
  await tick();
  setInterval(tick, POLL_INTERVAL_MS);
}

main().catch((err) => {
  console.error("Keeper failed:", err);
  process.exit(1);
});
