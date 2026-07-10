import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DecentralizedLottery } from "../target/types/decentralized_lottery";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
  SYSVAR_SLOT_HASHES_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAccount,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { assert } from "chai";
import { BN } from "bn.js";

describe("decentralized-lottery lifecycle", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.DecentralizedLottery as Program<DecentralizedLottery>;
  const admin = (provider.wallet as anchor.Wallet).payer;

  // PDAs
  let globalConfigPda: PublicKey;
  let lotteryPda: PublicKey;
  let lotteryVaultPda: PublicKey;

  // Token
  let usdcMint: PublicKey;
  let adminUsdcAta: PublicKey;
  let treasuryUsdcAta: PublicKey;

  // Buyers
  let buyer1: Keypair;
  let buyer2: Keypair;
  let buyer1UsdcAta: PublicKey;
  let buyer2UsdcAta: PublicKey;

  // Lottery params
  const ticketPrice = new BN(10_000_000); // 10 USDC
  const nonce = new BN(1);

  // Track ticket PDAs
  let ticket1Pda: PublicKey;
  let ticket2Pda: PublicKey;

  before(async () => {
    // Derive global config PDA — seed is "global_config_v2"
    [globalConfigPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("global_config_v2")],
      program.programId
    );

    // Create a test USDC mint (6 decimals)
    usdcMint = await createMint(
      provider.connection,
      admin,
      admin.publicKey,
      null,
      6
    );

    // Admin ATA
    adminUsdcAta = getAssociatedTokenAddressSync(
      usdcMint,
      admin.publicKey,
      false,
      undefined,
      undefined,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID
    );
    await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      admin.publicKey
    );

    // Treasury ATA — owned by the globalConfig PDA
    treasuryUsdcAta = getAssociatedTokenAddressSync(
      usdcMint,
      globalConfigPda,
      true,
      undefined,
      undefined,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID
    );
    await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      globalConfigPda,
      true
    );

    // Fund admin with USDC
    await mintTo(
      provider.connection,
      admin,
      usdcMint,
      adminUsdcAta,
      admin,
      1_000_000_000 // 1000 USDC
    );

    // Create buyers and fund them
    buyer1 = Keypair.generate();
    buyer2 = Keypair.generate();

    await provider.connection.requestAirdrop(buyer1.publicKey, 2 * LAMPORTS_PER_SOL);
    await provider.connection.requestAirdrop(buyer2.publicKey, 2 * LAMPORTS_PER_SOL);

    buyer1UsdcAta = getAssociatedTokenAddressSync(
      usdcMint,
      buyer1.publicKey,
      false,
      undefined,
      undefined,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID
    );
    buyer2UsdcAta = getAssociatedTokenAddressSync(
      usdcMint,
      buyer2.publicKey,
      false,
      undefined,
      undefined,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID
    );

    await getOrCreateAssociatedTokenAccount(provider.connection, admin, usdcMint, buyer1.publicKey);
    await getOrCreateAssociatedTokenAccount(provider.connection, admin, usdcMint, buyer2.publicKey);

    await mintTo(provider.connection, admin, usdcMint, buyer1UsdcAta, admin, 500_000_000);
    await mintTo(provider.connection, admin, usdcMint, buyer2UsdcAta, admin, 500_000_000);
  });

  it("initializes the global config", async () => {
    await program.methods
      .initialize()
      .accounts({
        globalConfig: globalConfigPda,
        admin: admin.publicKey,
        usdcMint,
        treasuryTokenAccount: treasuryUsdcAta,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const config = await program.account.globalConfig.fetch(globalConfigPda);
    assert.equal(config.admin.toBase58(), admin.publicKey.toBase58());
    assert.equal(config.treasuryFeePercentage, 250);
    assert.isFalse(config.isPaused);
    assert.notEqual(config.bump, 0, "bump should be set, not 0");
    assert.ok(config.minTicketPrice.gt(new BN(0)), "min ticket price should be set");
  });

  it("creates a lottery", async () => {
    // Short draw time for testing (2 minutes from now, above min_draw_duration of 60s)
    const drawTime = new BN(Math.floor(Date.now() / 1000) + 120);

    [lotteryPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lottery"), admin.publicKey.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
      program.programId
    );

    // Derive the lottery vault ATA (owned by lotteryPda)
    lotteryVaultPda = getAssociatedTokenAddressSync(
      usdcMint,
      lotteryPda,
      true,
      undefined,
      undefined,
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID
    );
    // Create the vault ATA before ticket purchases
    await getOrCreateAssociatedTokenAccount(
      provider.connection,
      admin,
      usdcMint,
      lotteryPda,
      true
    );

    await program.methods
      .createLottery({ daily: {} }, ticketPrice, drawTime, new BN(0), nonce)
      .accounts({
        lotteryAccount: lotteryPda,
        globalConfig: globalConfigPda,
        creator: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.deepEqual(lottery.state, { created: {} });
    assert.ok(lottery.ticketPrice.eq(ticketPrice));
    assert.equal(lottery.totalTickets, 0);
    assert.equal(lottery.nonce, nonce.toNumber());
  });

  it("opens the lottery for ticket purchases", async () => {
    await program.methods
      .transitionState({ open: {} })
      .accounts({
        lotteryAccount: lotteryPda,
        globalConfig: globalConfigPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.deepEqual(lottery.state, { open: {} });
  });

  it("buyer1 purchases a ticket", async () => {
    ticket1Pda = PublicKey.findProgramAddressSync(
      [Buffer.from("ticket"), lotteryPda.toBuffer(), new BN(1).toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];

    const buyer1BalBefore = (await getAccount(provider.connection, buyer1UsdcAta)).amount;

    await program.methods
      .buyTicket()
      .accounts({
        lotteryAccount: lotteryPda,
        ticketAccount: ticket1Pda,
        globalConfig: globalConfigPda,
        user: buyer1.publicKey,
        userTokenAccount: buyer1UsdcAta,
        lotteryTokenAccount: lotteryVaultPda,
        usdcMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer1])
      .rpc();

    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.equal(lottery.totalTickets, 1);
    assert.ok(lottery.prizePool.eq(ticketPrice));

    const buyer1BalAfter = (await getAccount(provider.connection, buyer1UsdcAta)).amount;
    assert.equal(buyer1BalBefore - buyer1BalAfter, BigInt(ticketPrice.toString()));

    const ticket = await program.account.ticketAccount.fetch(ticket1Pda);
    assert.equal(ticket.buyer.toBase58(), buyer1.publicKey.toBase58());
    assert.equal(ticket.id, 1);
  });

  it("buyer2 purchases a ticket", async () => {
    ticket2Pda = PublicKey.findProgramAddressSync(
      [Buffer.from("ticket"), lotteryPda.toBuffer(), new BN(2).toArrayLike(Buffer, "le", 8)],
      program.programId
    )[0];

    await program.methods
      .buyTicket()
      .accounts({
        lotteryAccount: lotteryPda,
        ticketAccount: ticket2Pda,
        globalConfig: globalConfigPda,
        user: buyer2.publicKey,
        userTokenAccount: buyer2UsdcAta,
        lotteryTokenAccount: lotteryVaultPda,
        usdcMint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([buyer2])
      .rpc();

    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.equal(lottery.totalTickets, 2);
    assert.ok(lottery.prizePool.eq(ticketPrice.mul(new BN(2))));
  });

  it("locks the lottery", async () => {
    await program.methods
      .transitionState({ locked: {} })
      .accounts({
        lotteryAccount: lotteryPda,
        globalConfig: globalConfigPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.deepEqual(lottery.state, { locked: {} });
  });

  it("transitions to drawing (after draw time)", async () => {
    // Wait until draw_time has passed
    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    const now = Math.floor(Date.now() / 1000);
    const waitMs = (lottery.drawTime.toNumber() - now + 2) * 1000;
    if (waitMs > 0) {
      console.log(`  Waiting ${waitMs / 1000}s for draw time...`);
      await new Promise((r) => setTimeout(r, waitMs));
    }

    await program.methods
      .transitionState({ drawing: {} })
      .accounts({
        lotteryAccount: lotteryPda,
        globalConfig: globalConfigPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const updated = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.deepEqual(updated.state, { drawing: {} });
  });

  it("transitions to awaiting randomness", async () => {
    await program.methods
      .transitionState({ awaitingRandomness: {} })
      .accounts({
        lotteryAccount: lotteryPda,
        globalConfig: globalConfigPda,
        admin: admin.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const updated = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.deepEqual(updated.state, { awaitingRandomness: {} });
  });

  it("settles randomness (fallback path) → Completed", async () => {
    // settle_randomness requires draw_time + 10 seconds
    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    const now = Math.floor(Date.now() / 1000);
    const waitMs = (lottery.drawTime.toNumber() + 10 - now + 2) * 1000;
    if (waitMs > 0) {
      console.log(`  Waiting ${waitMs / 1000}s for settle delay...`);
      await new Promise((r) => setTimeout(r, waitMs));
    }

    await program.methods
      .settleRandomness()
      .accounts({
        lotteryAccount: lotteryPda,
        recentBlockhashes: SYSVAR_SLOT_HASHES_PUBKEY,
        clock: anchor.web3.SYSVAR_CLOCK_PUBKEY,
        caller: admin.publicKey,
      })
      .rpc();

    const updated = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.deepEqual(updated.state, { completed: {} });
    assert.isTrue(updated.randomnessFulfilled);
    assert.isNotNull(updated.vrfRandomness);
  });

  it("selects the winner", async () => {
    // Determine which ticket won based on the randomness
    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    const randomnessBytes = lottery.vrfRandomness as number[];
    const randomValue = randomnessBytes
      .slice(0, 8)
      .reduce((acc: bigint, b: number, i: number) => acc | (BigInt(b) << BigInt(8 * i)), 0n);
    const winningId = Number(randomValue % BigInt(lottery.totalTickets)) + 1;
    const winningTicketPda = winningId === 1 ? ticket1Pda : ticket2Pda;
    const winnerKeypair = winningId === 1 ? buyer1 : buyer2;
    const winnerAta = winningId === 1 ? buyer1UsdcAta : buyer2UsdcAta;

    console.log(`  Winner: ticket ${winningId} (buyer${winningId})`);

    await program.methods
      .selectWinner()
      .accounts({
        lotteryAccount: lotteryPda,
        winningTicketAccount: winningTicketPda,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const updated = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.isNotNull(updated.winningTicket);
    assert.equal(updated.winningTicket?.toBase58(), winningTicketPda.toBase58());

    // Save for the claim test
    (globalThis as any).winnerKeypair = winnerKeypair;
    (globalThis as any).winnerAta = winnerAta;
    (globalThis as any).winningTicketPda = winningTicketPda;
  });

  it("winner claims the prize (with 2.5% treasury fee)", async () => {
    const winnerKeypair: Keypair = (globalThis as any).winnerKeypair;
    const winnerAta: PublicKey = (globalThis as any).winnerAta;
    const winningTicketPda: PublicKey = (globalThis as any).winningTicketPda;

    const winnerBalBefore = (await getAccount(provider.connection, winnerAta)).amount;
    const treasuryBalBefore = (await getAccount(provider.connection, treasuryUsdcAta)).amount;

    const lottery = await program.account.lotteryAccount.fetch(lotteryPda);
    const prizePool = BigInt(lottery.prizePool.toString());
    const expectedFee = (prizePool * 250n) / 10000n; // 2.5%
    const expectedPayout = prizePool - expectedFee;

    await program.methods
      .claimPrize()
      .accounts({
        lotteryAccount: lotteryPda,
        ticketAccount: winningTicketPda,
        globalConfig: globalConfigPda,
        winner: winnerKeypair.publicKey,
        lotteryTokenAccount: lotteryVaultPda,
        winnerTokenAccount: winnerAta,
        treasuryTokenAccount: treasuryUsdcAta,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([winnerKeypair])
      .rpc();

    const winnerBalAfter = (await getAccount(provider.connection, winnerAta)).amount;
    const treasuryBalAfter = (await getAccount(provider.connection, treasuryUsdcAta)).amount;

    // Verify payouts
    assert.equal(winnerBalAfter - winnerBalBefore, expectedPayout, "winner payout mismatch");
    assert.equal(treasuryBalAfter - treasuryBalBefore, expectedFee, "treasury fee mismatch");

    // Verify lottery marked as claimed (bit 2 of flags)
    const updated = await program.account.lotteryAccount.fetch(lotteryPda);
    const isClaimed = (updated.flags & 0b100) !== 0;
    assert.isTrue(isClaimed, "lottery should be marked claimed");

    console.log(`  Prize pool: ${prizePool / 1_000_000n} USDC`);
    console.log(`  Treasury fee (2.5%): ${expectedFee / 1_000_000n} USDC`);
    console.log(`  Winner payout: ${expectedPayout / 1_000_000n} USDC`);
  });
});
