/**
 * Lottery Keeper — auto-schedules and advances all lottery lifecycles.
 *
 * Responsibilities:
 *   1. On startup, ensures all 3 types (Daily, Weekly, Monthly) have an active lottery
 *   2. Advances lotteries past their draw_time: Open→Locked→Drawing→Awaiting→Settle→SelectWinner
 *   3. After a winner is selected, auto-creates the successor lottery for that type
 *
 * The keeper is the ONLY entity that creates lotteries. Players never schedule draws.
 *
 * Environment:
 *   KEEPER_SPEED=fast         → daily=5min, weekly=15min, monthly=30min (testing)
 *   KEEPER_SPEED=production   → daily=24h, weekly=7d, monthly=30d (default)
 *
 * Run: npm run keeper
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
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { BN } from "bn.js";

const POLL_INTERVAL_MS = 15_000; // 15 seconds

// Draw intervals per type, by speed mode
const SCHEDULE = {
  fast: { daily: 5 * 60, weekly: 15 * 60, monthly: 30 * 60 },
  production: { daily: 24 * 60 * 60, weekly: 7 * 24 * 60 * 60, monthly: 30 * 24 * 60 * 60 },
};
type LotteryTypeKey = "daily" | "weekly" | "monthly";

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.DecentralizedLottery as Program<DecentralizedLottery>;
  const admin = (provider.wallet as anchor.Wallet).payer;
  const connection = provider.connection;

  const speed = (process.env.KEEPER_SPEED || "production") as "fast" | "production";
  const schedule = SCHEDULE[speed] || SCHEDULE.production;

  console.log("╔══════════════════════════════════════╗");
  console.log("║       Lottery Keeper (auto)          ║");
  console.log("╚══════════════════════════════════════╝");
  console.log("Admin:", admin.publicKey.toBase58());
  console.log("Program:", program.programId.toBase58());
  console.log(`Speed: ${speed}`);
  console.log(`  Daily:  draws every ${schedule.daily / 60}min`);
  console.log(`  Weekly: draws every ${schedule.weekly / 60}min`);
  console.log(`  Monthly: draws every ${schedule.monthly / 60}min`);
  console.log(`Polling every ${POLL_INTERVAL_MS / 1000}s...\n`);

  // Derive global config PDA
  const [globalConfigPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("global_config_v2")],
    program.programId
  );

  // Verify admin
  let usdcMint: PublicKey;
  try {
    const config = await (program.account as any).globalConfig.fetch(globalConfigPda);
    if (config.admin.toBase58() !== admin.publicKey.toBase58()) {
      console.error("ERROR: Wallet is not the admin.");
      process.exit(1);
    }
    usdcMint = config.usdcMint;
    console.log("✓ Admin verified");
    console.log("✓ USDC mint:", usdcMint.toBase58(), "\n");
  } catch {
    console.error("GlobalConfig not found. Run setup:devnet first.");
    process.exit(1);
  }

  // ── Helper: create a lottery of a given type ──────────────────────────
  async function createLotteryOfType(typeKey: LotteryTypeKey): Promise<PublicKey | null> {
    const typeEnum = { [typeKey]: {} };
    const nonce = new BN(Date.now() + Math.floor(Math.random() * 1000));
    const [lotteryPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lottery"), admin.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    const ticketPrice = new BN(typeKey === "daily" ? 1_000_000 : typeKey === "weekly" ? 5_000_000 : 10_000_000);
    const drawTime = new BN(Math.floor(Date.now() / 1000) + schedule[typeKey]);

    // Create vault ATA
    try {
      await getOrCreateAssociatedTokenAccount(connection, admin, usdcMint, lotteryPda, true);
    } catch {
      // May already exist
    }

    try {
      await (program.methods as any)
        .createLottery(typeEnum, ticketPrice, drawTime, new BN(0), nonce)
        .accounts({
          lotteryAccount: lotteryPda,
          globalConfig: globalConfigPda,
          creator: admin.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      // Open it immediately
      await (program.methods as any)
        .transitionState({ open: {} })
        .accounts({
          lotteryAccount: lotteryPda,
          globalConfig: globalConfigPda,
          admin: admin.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log(`  ✦ Created ${typeKey} lottery: ${lotteryPda.toBase58().slice(0, 12)}... (${ticketPrice.toNumber() / 1e6} USDC, draws in ${schedule[typeKey] / 60}min)`);
      return lotteryPda;
    } catch (err: any) {
      console.error(`  ✗ Failed to create ${typeKey} lottery:`, err.message?.slice(0, 100));
      return null;
    }
  }

  // ── Helper: ensure all 3 types have an active lottery ─────────────────
  async function ensureAllTypesActive(allLotteries: any[]) {
    const activeByType: Record<string, boolean> = { daily: false, weekly: false, monthly: false };
    for (const { account } of allLotteries) {
      const state = Object.keys(account.state)[0];
      const type = Object.keys(account.lotteryType)[0];
      if ((state === "open" || state === "created") && type in activeByType) {
        activeByType[type] = true;
      }
    }

    for (const typeKey of ["daily", "weekly", "monthly"] as LotteryTypeKey[]) {
      if (!activeByType[typeKey]) {
        console.log(`[scheduler] No active ${typeKey} lottery — creating one...`);
        await createLotteryOfType(typeKey);
      }
    }
  }

  // ── Helper: create successor after a completed lottery ────────────────
  async function createSuccessor(typeKey: LotteryTypeKey) {
    console.log(`[scheduler] Creating successor ${typeKey} lottery...`);
    await createLotteryOfType(typeKey);
  }

  // ── Startup: ensure all types exist ───────────────────────────────────
  console.log("--- Startup: ensuring all lottery types are active ---");
  const initialLotteries = await (program.account as any).lotteryAccount.all();
  await ensureAllTypesActive(initialLotteries);
  console.log("");

  // ── Main tick loop ────────────────────────────────────────────────────
  async function tick() {
    const now = Math.floor(Date.now() / 1000);
    let allLotteries: any[];

    try {
      allLotteries = await (program.account as any).lotteryAccount.all();
    } catch (err) {
      console.error("Failed to fetch lotteries:", err);
      return;
    }

    // Ensure all types always have an active lottery
    await ensureAllTypesActive(allLotteries);

    // Re-fetch after potential creations
    try {
      allLotteries = await (program.account as any).lotteryAccount.all();
    } catch {
      return;
    }

    for (const { publicKey, account } of allLotteries) {
      const state = Object.keys(account.state)[0];
      const drawTime = account.drawTime;
      const typeKey = Object.keys(account.lotteryType)[0] as LotteryTypeKey;
      const label = `${publicKey.toBase58().slice(0, 8)}`;

      // Only process lotteries past their draw time
      if (now < drawTime && state !== "awaitingRandomness") continue;

      try {
        if (state === "open") {
          console.log(`[${label}] ${typeKey} → Locked`);
          await (program.methods as any)
            .transitionState({ locked: {} })
            .accounts({ lotteryAccount: publicKey, globalConfig: globalConfigPda, admin: admin.publicKey, systemProgram: SystemProgram.programId })
            .rpc();

        } else if (state === "locked") {
          console.log(`[${label}] ${typeKey} → Drawing`);
          await (program.methods as any)
            .transitionState({ drawing: {} })
            .accounts({ lotteryAccount: publicKey, globalConfig: globalConfigPda, admin: admin.publicKey, systemProgram: SystemProgram.programId })
            .rpc();

        } else if (state === "drawing") {
          console.log(`[${label}] ${typeKey} → AwaitingRandomness`);
          await (program.methods as any)
            .transitionState({ awaitingRandomness: {} })
            .accounts({ lotteryAccount: publicKey, globalConfig: globalConfigPda, admin: admin.publicKey, systemProgram: SystemProgram.programId })
            .rpc();

        } else if (state === "awaitingRandomness") {
          if (now < drawTime + 10) continue;

          console.log(`[${label}] ${typeKey} → Settling randomness`);
          await (program.methods as any)
            .settleRandomness()
            .accounts({ lotteryAccount: publicKey, recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY, clock: SYSVAR_CLOCK_PUBKEY, caller: admin.publicKey })
            .rpc();

          // Select winner
          const lottery = await (program.account as any).lotteryAccount.fetch(publicKey);
          const randomness = lottery.vrfRandomness;
          if (!randomness) continue;

          const totalTickets = lottery.totalTickets;
          if (totalTickets === 0) {
            console.log(`[${label}] No tickets sold — marking expired`);
            continue;
          }

          const randomValue = randomness.slice(0, 8).reduce(
            (acc: bigint, b: number, i: number) => acc | (BigInt(b) << BigInt(8 * i)), 0n
          );
          const winningId = Number(randomValue % BigInt(totalTickets)) + 1;
          const ticketPda = PublicKey.findProgramAddressSync(
            [Buffer.from("ticket"), publicKey.toBuffer(), new BN(winningId).toArrayLike(Buffer, "le", 8)],
            program.programId
          )[0];

          console.log(`[${label}] ${typeKey} → Selecting winner: ticket #${winningId}`);
          await (program.methods as any)
            .selectWinner()
            .accounts({ lotteryAccount: publicKey, winningTicketAccount: ticketPda, systemProgram: SystemProgram.programId })
            .rpc();

          console.log(`[${label}] ${typeKey} ✓ Draw complete`);

          // ── Auto-create the successor lottery for this type ──
          await createSuccessor(typeKey);
        }
      } catch (err: any) {
        const msg = err?.message || String(err);
        if (msg.includes("0x1") || msg.toLowerCase().includes("insufficient")) {
          console.error(`[${label}] Insufficient funds. Top up admin wallet.`);
          process.exit(1);
        }
        console.error(`[${label}] Error (${state}):`, msg.slice(0, 120));
      }
    }
  }

  await tick();
  setInterval(tick, POLL_INTERVAL_MS);
}

main().catch((err) => {
  console.error("Keeper failed:", err);
  process.exit(1);
});
