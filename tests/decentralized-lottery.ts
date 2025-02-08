import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DecentralizedLottery } from "../target/types/decentralized_lottery";
import { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, createMint, createAccount, mintTo, getMint, getAccount } from "@solana/spl-token";
import { assert } from "chai";
import { BN } from "bn.js";

describe("decentralized-lottery", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.DecentralizedLottery as Program<DecentralizedLottery>;
  const provider = anchor.AnchorProvider.local();
  const payer = provider.wallet as anchor.Wallet;

  // Accounts
  let configPda: PublicKey;
  let treasuryPda: PublicKey;
  let lotteryIdGenerator: Keypair; // Example keypair for lottery ID - replace with better approach if needed
  let lotteryPda: PublicKey;
  let ticketPda: PublicKey;

  // Token & ATA Accounts
  let usdcMint: PublicKey;
  let operatorUsdcAta: PublicKey;
  let lotteryUsdcAta: PublicKey;
  let treasuryUsdcAta: PublicKey;
  let purchaserUsdcAta: PublicKey;

  // Operator & Purchaser
  const authorizedOperator = payer.payer; // Using payer as authorized operator for testing
  const purchaser = Keypair.generate();

  const lotteryType = { daily: {} };
  const ticketNumbers = [1, 2, 3, 4, 5, 6];

  before(async () => {
    // 1. Derive PDAs
    const configPdaSeed = anchor.utils.bytes.utf8.encode("global_config");
    const treasuryPdaSeed = anchor.utils.bytes.utf8.encode("treasury");
    [configPda] = PublicKey.findProgramAddressSync([configPdaSeed], program.programId);
    [treasuryPda] = PublicKey.findProgramAddressSync([treasuryPdaSeed], program.programId);

    lotteryIdGenerator = Keypair.generate(); // Generate keypair for lottery ID

    // 2. Create USDC Mint
    usdcMint = await createMint(
      provider.connection,
      payer,
      authorizedOperator.publicKey, // Mint authority (can be program or operator)
      null,                         // Freeze authority (null for none)
      6,                            // Decimals
      TOKEN_PROGRAM_ID
    );

    // 3. Get or Create ATAs
    operatorUsdcAta = getAssociatedTokenAddressSync(usdcMint, authorizedOperator.publicKey, false, TOKEN_PROGRAM_ID);
    lotteryUsdcAta = getAssociatedTokenAddressSync(usdcMint, program.programId, true, TOKEN_PROGRAM_ID); // PDA owner
    treasuryUsdcAta = getAssociatedTokenAddressSync(usdcMint, treasuryPda, true, TOKEN_PROGRAM_ID);       // PDA owner
    purchaserUsdcAta = getAssociatedTokenAddressSync(usdcMint, purchaser.publicKey, false, TOKEN_PROGRAM_ID);

    await Promise.all([
        provider.connection.requestAirdrop(purchaser.publicKey, LAMPORTS_PER_SOL),
        createAccount(provider.connection, payer, usdcMint, purchaser.publicKey), // Purchaser USDC ATA
        createAccount(provider.connection, payer, usdcMint, program.programId),    // Lottery USDC ATA
        createAccount(provider.connection, payer, usdcMint, treasuryPda),          // Treasury USDC ATA
        createAccount(provider.connection, payer, usdcMint, authorizedOperator.publicKey), // Operator USDC ATA
    ]);


    // 4. Mint USDC to Operator and Purchaser
    const mintAmount = new BN(1000 * 10**6); // 1000 USDC
    await mintTo(provider.connection, payer, usdcMint, operatorUsdcAta, authorizedOperator, mintAmount.mul(new BN(10)));
    await mintTo(provider.connection, payer, usdcMint, purchaserUsdcAta, authorizedOperator, mintAmount);

    // 5. Initialize Config and Treasury
    await program.methods.initializeConfig(authorizedOperator.publicKey)
      .accounts({
        config: configPda,
        payer: authorizedOperator.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .signers([authorizedOperator])
      .rpc();

      // Treasury is initialized implicitly upon first deposit (in execute_draw)
  });

  it("Initialize Config and Treasury", async () => {
    const configAccount = await program.account.globalConfig.fetch(configPda);
    assert.isTrue(configAccount.authorizedOperator.equals(authorizedOperator.publicKey));
  });


  it("Create Lottery", async () => {
    const lotteryId = 1; // Example lottery ID, replace with dynamic generation if needed
    const lotteryPdaSeed = anchor.utils.bytes.utf8.encode("lottery");
    const lotteryIdBytes = Buffer.from(lotteryId.toString());
    [lotteryPda] = PublicKey.findProgramAddressSync([lotteryPdaSeed, lotteryIdBytes], program.programId);

    await program.methods.createLottery(lotteryType)
      .accounts({
        lottery: lotteryPda,
        config: configPda,
        operator: authorizedOperator.publicKey,
        systemProgram: SystemProgram.programId,
        clock: SYSVAR_CLOCK_PUBKEY,
        lotteryIdGenerator: lotteryIdGenerator.publicKey, // Using keypair pubkey as seed - simplistic
      })
      .signers([authorizedOperator, lotteryIdGenerator]) // Sign with operator and lotteryIdGenerator
      .rpc();

    const lotteryAccount = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.isTrue(lotteryAccount.lotteryType.daily !== undefined);
    assert.isTrue(lotteryAccount.lotteryState.created !== undefined);
    assert.equal(lotteryAccount.ticketPrice.toString(), (1 * 10**6).toString()); // 1 USDC
  });


  it("Buy Ticket", async () => {
    const lotteryAccountBefore = await program.account.lotteryAccount.fetch(lotteryPda);
    const purchaserUsdcAtaBefore = await getAccount(provider.connection, purchaserUsdcAta);

    const ticketSeed = anchor.utils.bytes.utf8.encode("ticket");
    const lotteryIdBytes = Buffer.from("1"); // Assuming lottery ID 1 from previous test
    const purchaserBytes = purchaser.publicKey.toBytes();
    [ticketPda] = PublicKey.findProgramAddressSync([ticketSeed, lotteryIdBytes, purchaserBytes], program.programId);


    await program.methods.buyTicket(new BN(1), ticketNumbers)
      .accounts({
        lottery: lotteryPda,
        ticket: ticketPda,
        purchaser: purchaser.publicKey,
        purchaserUsdcAta: purchaserUsdcAta,
        lotteryUsdcAta: lotteryUsdcAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([purchaser])
      .rpc();

    const lotteryAccountAfter = await program.account.lotteryAccount.fetch(lotteryPda);
    const ticketAccount = await program.account.ticketAccount.fetch(ticketPda);
    const purchaserUsdcAtaAfter = await getAccount(provider.connection, purchaserUsdcAta);

    assert.isTrue(lotteryAccountAfter.prizePool.gt(lotteryAccountBefore.prizePool));
    assert.equal(lotteryAccountAfter.prizePool.toString(), (1 * 10**6).toString()); // Prize pool increased by ticket price
    assert.isTrue(ticketAccount.purchaser.equals(purchaser.publicKey));
    assert.deepEqual(ticketAccount.ticketNumbers, ticketNumbers);
    assert.equal(purchaserUsdcAtaAfter.amount.toString(), purchaserUsdcAtaBefore.amount.sub(new BN(1 * 10**6)).toString()); // Purchaser USDC balance decreased
  });


  it("Execute Draw (Placeholder - Needs Pyth Price Feed Mocking)", async () => {
    // This test is a placeholder and needs Pyth price feed mocking for actual randomness in tests.
    // For now, it just checks if the instruction executes without errors and transitions lottery state.

    const treasuryAccountBefore = await program.account.treasuryAccount.fetch(treasuryPda);
    const lotteryAccountBeforeDraw = await program.account.lotteryAccount.fetch(lotteryPda);

    // Mock Pyth Price Feed Account (Replace with actual mocking/setup for real tests)
    const pythPriceFeed = Keypair.generate(); // In real tests, use a proper Pyth Price Feed account address
    // Assume price feed account is set up with some dummy data for testing purposes.

    await program.methods.executeDraw(new BN(1))
      .accounts({
        lottery: lotteryPda,
        config: configPda,
        treasury: treasuryPda,
        pythPriceFeed: pythPriceFeed.publicKey, // Use mock Pyth price feed here
        lotteryUsdcAta: lotteryUsdcAta,
        treasuryUsdcAta: treasuryUsdcAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        clock: SYSVAR_CLOCK_PUBKEY,
      })
      .signers([authorizedOperator]) // Or lottery PDA signer if needed
      .rpc();

    const lotteryAccountAfterDraw = await program.account.lotteryAccount.fetch(lotteryPda);
    const treasuryAccountAfter = await program.account.treasuryAccount.fetch(treasuryPda);


    assert.isTrue(lotteryAccountAfterDraw.lotteryState.completed !== undefined);
    assert.isTrue(lotteryAccountAfterDraw.winningNumbers !== null);
    assert.isTrue(treasuryAccountAfter.balance.gt(treasuryAccountBefore.balance)); // Treasury balance increased by fee
    assert.equal(treasuryAccountAfter.balance.toString(), (1 * 10**6 * 0.025).toString()); // Treasury fee collected (2.5% of ticket price)

    // In real tests, add assertions to check prize distribution and winning ticket logic.
    // This requires more setup including mocking Pyth price feed to control random numbers and determine winners predictably for testing.
  });

  it("Distribute Prize (Placeholder - Needs Winning Logic & More Tickets)", async () => {
    // This is a placeholder. Need to buy multiple tickets, mock draw to ensure a winning ticket, and then test distribution.

    // For now, assume ticketPda is a winning ticket (for testing prize distribution logic)
    const winnerUsdcAta = purchaserUsdcAta; // Winner ATA is purchaser ATA for simplicity in this example

     const lotteryAccountBeforeDistribute = await program.account.lotteryAccount.fetch(lotteryPda);
     const winnerUsdcAtaBalanceBefore = await getAccount(provider.connection, winnerUsdcAta);


    await program.methods.distributePrize(new BN(1))
      .accounts({
        lottery: lotteryPda,
        winnerTicket: ticketPda,
        winner: purchaser.publicKey,
        winnerUsdcAta: winnerUsdcAta,
        lotteryUsdcAta: lotteryUsdcAta,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([purchaser]) // Winner signs
      .rpc();

    const ticketAccountAfterPrizeClaim = await program.account.ticketAccount.fetch(ticketPda);
    const lotteryAccountAfterDistribute = await program.account.lotteryAccount.fetch(lotteryPda);
    const winnerUsdcAtaBalanceAfter = await getAccount(provider.connection, winnerUsdcAta);


    assert.isTrue(ticketAccountAfterPrizeClaim.prizeClaimed);
    assert.isTrue(lotteryAccountAfterDistribute.prizePool.lt(lotteryAccountBeforeDistribute.prizePool)); // Prize pool reduced
    assert.isTrue(winnerUsdcAtaBalanceAfter.amount.gt(winnerUsdcAtaBalanceBefore.amount)); // Winner balance increased

    // In real tests, assert specific prize amount based on ticket tier and lottery prize pool.
  });

  // Add more tests for:
  // - Recycle Unclaimed Prize (after time passes and no claim)
  // - Treasury Withdrawal (after timelock and by authorized operator)
  // - Error cases and validations

});