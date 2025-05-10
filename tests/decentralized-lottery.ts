import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DecentralizedLottery } from "../target/types/decentralized_lottery";
import { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
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
    await program.methods.initializeConfig(authorizedOperator.publicKey, treasuryUsdcAta, 500, usdcMint)
      .accounts({
        globalConfig: configPda,
        admin: authorizedOperator.publicKey,
        treasuryTokenAccount: treasuryUsdcAta,
        usdcMint: usdcMint,
        systemProgram: SystemProgram.programId,
      })
      .signers([authorizedOperator as any])
      .rpc();

    const globalConfigAccount = await program.account.globalConfig.fetch(configPda);
    assert.equal(globalConfigAccount.admin.toString(), authorizedOperator.publicKey.toString());
    assert.equal(globalConfigAccount.treasuryTokenAccount.toString(), treasuryUsdcAta.toString());
    assert.equal(globalConfigAccount.treasuryFeePercentage, 500);
    assert.equal(globalConfigAccount.usdcMint.toString(), usdcMint.toString());
  });


  it("Create Lottery", async () => {
    const lotteryType = { daily: {} };
    const ticketPrice = new BN(1 * 10**6); // 1 USDC
    const drawTime = new BN(Math.floor((new Date().getTime() + 24 * 60 * 60 * 1000) / 1000)); // Tomorrow
    const prizePool = new BN(100 * 10**6); // 100 USDC

    // Validate inputs before transaction
    if (ticketPrice.lte(new BN(0))) {
      throw new Error("Ticket price must be greater than 0");
    }
    if (drawTime.lte(new BN(Math.floor(new Date().getTime() / 1000)))) {
      throw new Error("Draw time must be in the future");
    }

    await program.methods.createLottery(lotteryType, ticketPrice, drawTime, prizePool)
      .accounts({
        lotteryAccount: lotteryPda,
        creator: authorizedOperator.publicKey,
        globalConfig: configPda,
        tokenMint: usdcMint,
        creatorTokenAccount: operatorUsdcAta,
        lotteryTokenAccount: lotteryUsdcAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([authorizedOperator as any])
      .rpc();

    const lotteryAccount = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.equal(lotteryAccount.lotteryType.daily, true);
    assert.equal(lotteryAccount.ticketPrice.toString(), ticketPrice.toString());
    assert.equal(lotteryAccount.drawTime.toString(), drawTime.toString());
    assert.equal(lotteryAccount.prizePool.toString(), prizePool.toString());
    assert.equal(lotteryAccount.state.created, true);
  });


  it("Buy Ticket", async () => {
    const lotteryAccountBefore = await program.account.lotteryAccount.fetch(lotteryPda);
    const purchaserUsdcAtaBefore = await getAccount(provider.connection, purchaserUsdcAta);

    const ticketSeed = anchor.utils.bytes.utf8.encode("ticket");
    const lotteryKey = lotteryPda.toBytes();
    const ticketIdBytes = Buffer.from(lotteryAccountBefore.lastTicketId.toString());
    [ticketPda] = PublicKey.findProgramAddressSync([ticketSeed, lotteryKey, ticketIdBytes], program.programId);

    await program.methods.buyTicket()
      .accounts({
        lotteryAccount: lotteryPda,
        ticketAccount: ticketPda,
        userTokenAccount: purchaserUsdcAta,
        lotteryTokenAccount: lotteryUsdcAta,
        buyer: purchaser.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([purchaser as any])
      .rpc();

    const lotteryAccountAfter = await program.account.lotteryAccount.fetch(lotteryPda);
    const ticketAccount = await program.account.ticketAccount.fetch(ticketPda);
    const purchaserUsdcAtaAfter = await getAccount(provider.connection, purchaserUsdcAta);

    assert.isTrue(lotteryAccountAfter.prizePool.gt(lotteryAccountBefore.prizePool));
    assert.equal(lotteryAccountAfter.prizePool.toString(), (1 * 10**6).toString()); // Prize pool increased by ticket price
    assert.isTrue(ticketAccount.buyer.equals(purchaser.publicKey));
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

  it("Add Authorized Operator", async () => {
    await program.methods.addAuthorizedOperator(authorizedOperator.publicKey)
      .accounts({
        globalConfig: configPda,
        admin: authorizedOperator.publicKey,
      })
      .signers([authorizedOperator as any])
      .rpc();

    const globalConfigAccount = await program.account.globalConfig.fetch(configPda);
    // Assuming authorizedOperators is an array in the account structure
    assert.equal(globalConfigAccount.authorizedOperators[0].toString(), authorizedOperator.publicKey.toString());
  });

  // Add more tests for:
  // - Recycle Unclaimed Prize (after time passes and no claim)
  // - Treasury Withdrawal (after timelock and by authorized operator)
  // - Error cases and validations

});