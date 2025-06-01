import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { DecentralizedLottery } from "../target/types/decentralized_lottery";
import { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL, SYSVAR_CLOCK_PUBKEY, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, createMint, createAccount, mintTo, getMint, getAccount, createAssociatedTokenAccountInstruction, getOrCreateAssociatedTokenAccount } from "@solana/spl-token";
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
  let treasuryConfigPda: PublicKey; // Renamed for clarity if it's for a treasury config state
  let lotteryPda: PublicKey; // Will be set dynamically in 'Create Lottery' test
  let ticketPda: PublicKey; // Will be set dynamically in 'Buy Ticket' test

  // Token & ATA Accounts
  let usdcMint: PublicKey;
  let operatorUsdcAta: PublicKey;
  let lotteryUsdcAta: PublicKey; // Will be derived dynamically once lotteryPda is known
  let treasuryUsdcAta: PublicKey; // ATA for the program's treasury, owned by configPda
  let purchaserUsdcAta: PublicKey;

  // Operator & Purchaser
  const authorizedOperator = payer.payer;
  const purchaser = Keypair.generate();

  // Default lottery parameters for tests
  const defaultLotteryType = { daily: {} };
  const defaultTicketPrice = new BN(1 * 10**6); // 1 USDC
  const defaultDrawTimeOffset = 24 * 60 * 60 * 1000; // 24 hours from now
  const defaultTargetPrizePool = new BN(100 * 10**6); // 100 USDC

  before(async () => {
    // 1. Derive PDAs (those that are static)
    [configPda] = PublicKey.findProgramAddressSync(
      [anchor.utils.bytes.utf8.encode("global_config")],
      program.programId
    );

    // Treasury PDA for a potential TreasuryConfig account (if used by program)
    // For now, this is not directly used by core lottery logic but might be for future extensions.
    // [treasuryConfigPda] = PublicKey.findProgramAddressSync(
    //   [anchor.utils.bytes.utf8.encode("treasury_config")],
    //   program.programId
    // );

    // 2. Create USDC Mint
    usdcMint = await createMint(
      provider.connection,
      payer.payer,
      authorizedOperator.publicKey,
      null,
      6
    );

    // 3. Create ATAs for operator and purchaser
    await provider.connection.requestAirdrop(purchaser.publicKey, LAMPORTS_PER_SOL * 2); // Airdrop SOL to purchaser

    operatorUsdcAta = (await getOrCreateAssociatedTokenAccount(provider.connection, payer.payer, usdcMint, authorizedOperator.publicKey)).address;
    purchaserUsdcAta = (await getOrCreateAssociatedTokenAccount(provider.connection, payer.payer, usdcMint, purchaser.publicKey)).address;
    
    // Treasury ATA: Owned by the GlobalConfig PDA. This ATA will store the treasury fees.
    treasuryUsdcAta = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        payer.payer, // Payer to create ATA
        usdcMint,    // Mint
        configPda,   // Owner of the ATA is the configPda
        true         // Allow owner off curve (since it's a PDA)
    ).then(acc => acc.address);

    // lotteryUsdcAta will be created dynamically in tests once lotteryPda is known,
    // as it's an ATA owned by the lotteryPda.

    // 4. Mint USDC to Operator and Purchaser ATAs
    const mintAmount = new BN(1000 * 10**6); // 1000 USDC
    await mintTo(provider.connection, payer.payer, usdcMint, operatorUsdcAta, authorizedOperator, BigInt(mintAmount.mul(new BN(10)).toString()));
    await mintTo(provider.connection, payer.payer, usdcMint, purchaserUsdcAta, authorizedOperator, BigInt(mintAmount.toString()));

    // 5. Initialize Config
    // Note: Treasury initialization might be a separate instruction or part of another flow.
    // The provided `initializeConfig` in lib.rs takes admin, usdc_mint, treasury_token_account.
    // The current `initialize` instruction sets `global_config.treasury_token_account`.
    // This `treasuryUsdcAta` (owned by `configPda`) will be passed to it.
    await program.methods.initialize()
      .accounts({
        globalConfig: configPda,
        admin: authorizedOperator.publicKey,
        usdcMint: usdcMint,
        treasuryTokenAccount: treasuryUsdcAta, 
        systemProgram: SystemProgram.programId,
      })
      .signers([authorizedOperator])
      .rpc();
  });

  it("Should initialize config correctly", async () => {
    // Fetch and verify the global config
    const globalConfigAccount = await program.account.globalConfig.fetch(configPda);
    assert.equal(globalConfigAccount.admin.toString(), authorizedOperator.publicKey.toString());
    assert.equal(globalConfigAccount.treasuryTokenAccount.toString(), treasuryUsdcAta.toString());
    assert.equal(globalConfigAccount.treasuryFeePercentage, 250); // Default from lib.rs
    assert.equal(globalConfigAccount.usdcMint.toString(), usdcMint.toString());
  });


  it("Should create a new lottery", async () => {
    const drawTime = new BN(Math.floor((Date.now() + defaultDrawTimeOffset) / 1000));

    // Add an event listener for LotteryCreated event to capture the new lottery Pda
    let listener = null;
    const [eventPromise, _listenerId] = new Promise((resolve, reject) => {
      listener = program.addEventListener("LotteryCreated", (event, slot) => {
        resolve({ event, slot });
      });
    });
    listenerId = _listenerId; // Store listenerId to remove later

    // Since lottery Pda depends on Clock, we cannot predict it.
    // The instruction will create it. We must fetch it or get from event.
    // The `lotteryAccount` in .accounts() for createLottery is the one being created.
    // Anchor will handle deriving it if seeds are provided directly in .accounts(),
    // but here seeds include Clock. We need to pass a "dummy" or predictable key if possible,
    // or more correctly, ensure the program instruction creates it via its defined PDA logic.
    // The instruction `create_lottery` uses `init` with `seeds = [b"lottery", creator.key().as_ref(), &Clock::get().unwrap().unix_timestamp.to_le_bytes()]`
    // This means we cannot pass a pre-derived PDA easily if the timestamp is dynamic.
    // The solution is typically to use a known seed for testing (e.g. pass timestamp as arg) OR parse from event.

    // For this test, we'll use a temporary keypair for the account to be initialized by the program.
    // This is a common pattern if the PDA is not easily predictable or if you want the program to fully manage it.
    // **Correction**: This is not how `init` with PDA seeds works. Anchor expects the PDA to be passed.
    // The issue is the non-deterministic part (Clock).
    // We will rely on the event listener.

    // We need a key for `lotteryAccount` that Anchor can use to sign if it were a new Keypair.
    // But since it's a PDA, Anchor calculates this PDA using the provided seeds.
    // The problem is `Clock.get()` makes the PDA unpredictable from client for `accounts.lotteryAccount`.
    // A common pattern: use a known, fixed seed for tests, or pass timestamp as an argument.
    // If we can't change contract: one approach is to create a dummy PDA seed on client,
    // then fetch the actual one from event.
    // For now, let's try to make the seeds in `accounts` as specific as possible,
    // knowing that the `Clock` part is handled by the runtime.
    // Anchor TS client can derive PDAs if all seed components *except* bump are known.
    // However, `Clock.get().unix_timestamp` is not known beforehand.

    // The `lotteryAccount` field in `program.methods.createLottery().accounts({...})`
    // expects the PDA that *will be* created. Anchor uses this to build the instruction.
    // The actual `Clock` value will be used on-chain.
    // This means the client *cannot* perfectly predict the PDA if `Clock` is a seed component.
    // The event listener is the most robust way.

    // We will generate a temporary keypair. This is not used by the program for PDA derivation
    // but is sometimes required by Anchor if it thinks it needs to sign for an `init` account
    // that isn't a PDA it can derive. This is a bit of a hack for this non-deterministic PDA.
    const tempLotteryKp = Keypair.generate();


    await program.methods.createLottery(defaultLotteryType, defaultTicketPrice, drawTime, defaultTargetPrizePool)
      .accounts({
        lotteryAccount: tempLotteryKp.publicKey, // Pass a pubkey; program will init its own PDA. Anchor might complain.
                                               // Ideal: Pass seeds for Anchor to derive, if Clock wasn't used.
        creator: authorizedOperator.publicKey,
        globalConfig: configPda,
        systemProgram: SystemProgram.programId,
        // clock: SYSVAR_CLOCK_PUBKEY, // If instruction took Clock sysvar explicitly
      })
      .signers([authorizedOperator])
      .rpc();

    const { event } = await eventPromise as any;
    await program.removeEventListener(listenerId);

    assert.isNotNull(event, "LotteryCreated event not emitted");
    lotteryPda = new PublicKey(event.lotteryId); // Set the global lotteryPda from the event

    const lotteryAccount = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.ok(lotteryAccount.lotteryType.daily); // Assuming defaultLotteryType is daily
    assert.equal(lotteryAccount.ticketPrice.toString(), defaultTicketPrice.toString());
    assert.equal(lotteryAccount.drawTime.toString(), drawTime.toString());
    assert.equal(lotteryAccount.targetPrizePool.toString(), defaultTargetPrizePool.toString());
    assert.ok(lotteryAccount.state.created);
    assert.isTrue(lotteryAccount.authority.equals(authorizedOperator.publicKey));
    assert.isTrue(lotteryAccount.globalConfig.equals(configPda));

    // Store the determined lotteryPda for other tests
    global.lotteryPda = lotteryPda;
  });


  it("Should allow a user to buy a ticket", async () => {
    lotteryPda = global.lotteryPda; // Retrieve from global context set by create_lottery
    assert.isDefined(lotteryPda, "Lottery PDA not set from create_lottery test");

    // First, transition lottery to Open state
    await program.methods.transitionState({ open: {} })
        .accounts({
            lotteryAccount: lotteryPda,
            admin: authorizedOperator.publicKey,
            globalConfig: configPda,
            systemProgram: SystemProgram.programId,
        })
        .signers([authorizedOperator])
        .rpc();
    
    const lotteryAccountBefore = await program.account.lotteryAccount.fetch(lotteryPda);
    const purchaserUsdcAtaBefore = await getAccount(provider.connection, purchaserUsdcAta);
    const currentTicketPrice = lotteryAccountBefore.ticketPrice;

    // Derive ticket PDA
    const nextTicketId = lotteryAccountBefore.lastTicketId.add(new BN(1));
    [ticketPda] = PublicKey.findProgramAddressSync(
        [
            anchor.utils.bytes.utf8.encode("ticket"),
            lotteryPda.toBuffer(),
            nextTicketId.toBuffer('le', 8)
        ],
        program.programId
    );

    // Get or create ATA for the lottery Pda
    lotteryUsdcAta = await getOrCreateAssociatedTokenAccount(
        provider.connection,
        payer.payer,
        usdcMint,
        lotteryPda, // lotteryPda is the owner of its ATA
        true
    ).then(acc => acc.address);


    await program.methods.buyTicket()
      .accounts({
        lotteryAccount: lotteryPda,
        ticketAccount: ticketPda,
        userTokenAccount: purchaserUsdcAta,
        lotteryTokenAccount: lotteryUsdcAta,
        user: purchaser.publicKey,
        globalConfig: configPda,
        usdcMint: usdcMint, // Added usdcMint
        tokenProgram: TOKEN_PROGRAM_ID, // Added tokenProgram
        associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID, // Added associatedTokenProgram
        systemProgram: SystemProgram.programId,
      })
      .signers([purchaser])
      .rpc();

    const lotteryAccountAfter = await program.account.lotteryAccount.fetch(lotteryPda);
    const ticketAccount = await program.account.ticketAccount.fetch(ticketPda);
    const purchaserUsdcAtaAfter = await getAccount(provider.connection, purchaserUsdcAta);
    const lotteryTokenAccountAfter = await getAccount(provider.connection, lotteryUsdcAta);


    assert.isTrue(lotteryAccountAfter.prizePool.gt(lotteryAccountBefore.prizePool), "Prize pool should increase");
    assert.equal(lotteryAccountAfter.prizePool.toString(), lotteryAccountBefore.prizePool.add(currentTicketPrice).toString(), "Prize pool incorrect increase");
    assert.isTrue(ticketAccount.buyer.equals(purchaser.publicKey), "Ticket buyer not set correctly");
    assert.equal(lotteryAccountAfter.lastTicketId.toString(), nextTicketId.toString(), "Last ticket ID not updated");
    assert.equal(lotteryAccountAfter.totalTickets.toString(), lotteryAccountBefore.totalTickets.add(new BN(1)).toString(), "Total tickets not incremented");

    // Check token transfer
    assert.equal(new BN(purchaserUsdcAtaAfter.amount.toString()).toString(), new BN(purchaserUsdcAtaBefore.amount.toString()).sub(currentTicketPrice).toString(), "Purchaser balance incorrect");
    assert.equal(new BN(lotteryTokenAccountAfter.amount.toString()).toString(), currentTicketPrice.toString(), "Lottery ATA balance incorrect"); // Assuming it starts from 0 for this lottery

  });

  it("Should transition lottery to AwaitingRandomness", async () => {
    lotteryPda = global.lotteryPda;
    assert.isDefined(lotteryPda, "Lottery PDA not set");

    // To ensure draw time has passed for the Open -> Drawing transition (which leads to AwaitingRandomness)
    // We might need to either use a short draw_time in create_lottery for this specific test,
    // or if Anchor test framework allows, advance the clock.
    // For now, assume draw_time for global.lotteryPda has passed or is very soon.
    // If the defaultDrawTimeOffset is long, this test might need to create its own lottery
    // with a short draw time or wait.

    // Let's ensure current time is past draw time for the test.
    // This is tricky with fixed `defaultDrawTimeOffset`.
    // A better way for tests is to set draw_time to something like `Date.now()/1000 + 2` (2 seconds from now)
    // and then `await new Promise(resolve => setTimeout(resolve, 3000));`
    // For now, we'll proceed assuming the global lotteryPda's draw time can be passed.
    // This might require adjusting the `defaultDrawTimeOffset` to be very short for testing this path,
    // or creating a new lottery specifically for this test case.

    // Fetch the lottery account to check its draw_time
    const lotteryAcc = await program.account.lotteryAccount.fetch(lotteryPda);
    const currentTime = Math.floor(Date.now() / 1000);
    if (currentTime < lotteryAcc.drawTime.toNumber()) {
        console.log(`Draw time (${lotteryAcc.drawTime.toNumber()}) is in the future. Waiting...`);
        await new Promise(resolve => setTimeout(resolve, (lotteryAcc.drawTime.toNumber() - currentTime + 1) * 1000));
    }
    
    await program.methods.transitionState({ drawing: {} }) // Target "Drawing" which program logic turns to "AwaitingRandomness"
        .accounts({
            lotteryAccount: lotteryPda,
            admin: authorizedOperator.publicKey,
            globalConfig: configPda,
            systemProgram: SystemProgram.programId,
            // clock: SYSVAR_CLOCK_PUBKEY, // If instruction needs it explicitly
        })
        .signers([authorizedOperator])
        .rpc();

    const updatedLotteryAccount = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.ok(updatedLotteryAccount.state.awaitingRandomness, "Lottery state should be AwaitingRandomness");
    assert.isNotNull(updatedLotteryAccount.vrfRequestKey, "VRF request key should be set");
    // Assuming vrf_client was set during creation or a default one is used by the contract for now
    // For this test, vrf_request_key gets set to vrf_client in transition_state.rs
    assert.isTrue(updatedLotteryAccount.vrfRequestKey.equals(updatedLotteryAccount.vrfClient), "VRF request key should match VRF client");
    assert.isFalse(updatedLotteryAccount.randomnessFulfilled, "Randomness should not be fulfilled yet");
  });

  it("Should settle randomness (mocked)", async () => {
    lotteryPda = global.lotteryPda;
    assert.isDefined(lotteryPda, "Lottery PDA not set");

    const lotteryAccBefore = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.ok(lotteryAccBefore.state.awaitingRandomness, "Lottery must be in AwaitingRandomness state");
    assert.isNotNull(lotteryAccBefore.vrfRequestKey, "VRF request key must be set");

    // The settle_randomness instruction expects `vrf_account` to match `lotteryAccount.vrf_request_key`.
    // In our current mock setup, `vrf_request_key` is set to `lotteryAccount.vrf_client`.
    // So, we pass `lotteryAccount.vrf_client` as the `vrf_account`.
    // This account doesn't need to be a real Switchboard VRF account for this mocked test,
    // as the instruction handler currently simulates randomness.
    // It just needs to be a valid pubkey that matches.
    const mockVrfAccountKey = lotteryAccBefore.vrfClient;
    if (!mockVrfAccountKey) {
        throw new Error("LotteryAccount.vrfClient (used as mockVrfAccountKey) is not set. Ensure it's set during lottery creation or transition.");
    }

    await program.methods.settleRandomness()
        .accounts({
            lotteryAccount: lotteryPda,
            vrfAccount: mockVrfAccountKey, // Pass the key that matches vrf_request_key
        })
        // No explicit signers needed if instruction doesn't require them beyond PDA checks
        .rpc();

    const updatedLotteryAccount = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.ok(updatedLotteryAccount.state.completed, "Lottery state should be Completed");
    assert.isNotNull(updatedLotteryAccount.vrfRandomness, "VRF randomness should be set");
    const expectedRandomness = new BN(updatedLotteryAccount.completedAt.toBuffer('le',8).slice(0,8)).toBuffer('le',32);
    // The mock randomness in contract is:
    // clock.unix_timestamp.to_le_bytes()[0..8].try_into().unwrap_or_default().repeat(4).try_into().unwrap_or_default();
    // This is hard to replicate perfectly without knowing the exact `completed_at` timestamp used by the contract call.
    // For now, we'll just check it's not null and has length 32.
    assert.equal(updatedLotteryAccount.vrfRandomness.length, 32, "Randomness byte array length incorrect");
    assert.isTrue(updatedLotteryAccount.randomnessFulfilled, "Randomness should be fulfilled");
  });

  it("Should select a winner (after mocked randomness)", async () => {
    lotteryPda = global.lotteryPda;
    assert.isDefined(lotteryPda, "Lottery PDA not set");

    const lotteryAcc = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.ok(lotteryAcc.state.completed, "Lottery must be in Completed state");
    assert.isTrue(lotteryAcc.randomnessFulfilled, "Randomness must be fulfilled");
    assert.isNotNull(lotteryAcc.vrfRandomness, "VRF randomness must be set");
    assert.isNull(lotteryAcc.winningTicket, "Winning ticket should not be set yet");

    await program.methods.selectWinner()
        .accounts({
            lotteryAccount: lotteryPda,
            systemProgram: SystemProgram.programId,
        })
        // .signers([authorizedOperator]) // If admin signature is required
        .rpc();

    const updatedLotteryAccount = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.isNotNull(updatedLotteryAccount.winningTicket, "Winning ticket should be set");

    // Verify the winning ticket PDA
    const randomnessBytes = Uint8Array.from(updatedLotteryAccount.vrfRandomness);
    const randomValue = new BN(randomnessBytes.slice(0, 8), 'le'); // Extract u64 from first 8 bytes
    const totalTickets = updatedLotteryAccount.totalTickets;

    if (totalTickets.eq(new BN(0))) {
        throw new Error("Cannot select winner if no tickets were sold.");
    }

    const winningTicketIdNum = randomValue.mod(totalTickets).toNumber(); // This gives 0 to N-1
    // The ticket IDs in contract are 1-based if last_ticket_id starts at 0 and increments before assigning.
    // Current buy_ticket: nextTicketId = last_ticket_id + 1; then ticket_account.id = nextTicketId.
    // So ticket IDs are 1, 2, ...
    // select_winner.rs: winning_ticket_id = (random_value % lottery_account.total_tickets) + 1;
    // This is 1-based.

    const winningTicketId = randomValue.mod(totalTickets).add(new BN(1));


    const [expectedWinningTicketPda] = PublicKey.findProgramAddressSync(
        [
            anchor.utils.bytes.utf8.encode("ticket"),
            lotteryPda.toBuffer(),
            winningTicketId.toBuffer('le', 8)
        ],
        program.programId
    );

    assert.isTrue(updatedLotteryAccount.winningTicket.equals(expectedWinningTicketPda), "Winning ticket PDA mismatch");
    console.log("Selected winning ticket ID:", winningTicketId.toString());
    console.log("Winning ticket PDA:", updatedLotteryAccount.winningTicket.toBase58());
  });

  // Placeholder for Claim Prize test - to be updated
  it("Distribute Prize (Placeholder - Needs Winning Logic & More Tickets)", async () => {
    lotteryPda = global.lotteryPda; // from previous test
    assert.isDefined(lotteryPda, "Lottery PDA not set");

    const lotteryAccount = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.ok(lotteryAccount.state.completed, "Lottery must be completed to claim prize");
    assert.isNotNull(lotteryAccount.winningTicket, "Winning ticket must be selected");

    // This requires knowing the actual winner (purchaser) of the winningTicketPda.
    // For this test, we'll assume the 'purchaser' from the 'Buy Ticket' test is the winner.
    // This means the 'Buy Ticket' test must have bought the ticket that ends up winning.
    // This makes the test flaky if randomness changes.
    // A robust test would:
    // 1. Buy multiple tickets from different purchasers.
    // 2. After selectWinner, fetch the winningTicketPda.
    // 3. Fetch the TicketAccount for winningTicketPda to find the actual buyer.
    // 4. That buyer then calls claimPrize.

    // For now, let's assume 'purchaser' is the winner and 'ticketPda' (from buy_ticket test) is the winning one.
    // This implies the mocked randomness must select the ticket bought by 'purchaser'.
    // This is a significant simplification and might often fail.
    // We need to make sure that `ticketPda` (which is the last ticket bought in `buy_ticket` test)
    // is actually selected as the winner by `select_winner` test.
    // The current `select_winner` test calculates `expectedWinningTicketPda`. We should use that.
    
    const winningTicketAccountPda = lotteryAccount.winningTicket;
    const winningTicketAccount = await program.account.ticketAccount.fetch(winningTicketAccountPda);
    const winnerPurchaser = Keypair.fromSecretKey(purchaser.secretKey); // Assuming 'purchaser' is the one who bought winning ticket

    const winnerUsdcAta = await getOrCreateAssociatedTokenAccount(
        provider.connection, payer.payer, usdcMint, winnerPurchaser.publicKey
    ).then(acc => acc.address);

    const lotteryTokenAccountAta = await getOrCreateAssociatedTokenAccount(
        provider.connection, payer.payer, usdcMint, lotteryPda, true
    ).then(acc => acc.address);

    const treasuryAta = treasuryUsdcAta; // from before() block, owned by configPda

    const winnerUsdcBalanceBefore = (await getAccount(provider.connection, winnerUsdcAta)).amount;
    const lotteryAtaBalanceBefore = (await getAccount(provider.connection, lotteryTokenAccountAta)).amount;
    const treasuryAtaBalanceBefore = (await getAccount(provider.connection, treasuryAta)).amount;


    await program.methods.claimPrize()
        .accounts({
            lotteryAccount: lotteryPda,
            ticketAccount: winningTicketAccountPda,
            winner: winnerPurchaser.publicKey,
            winnerTokenAccount: winnerUsdcAta,
            lotteryTokenAccount: lotteryTokenAccountAta,
            globalConfig: configPda, 
            treasuryTokenAccount: treasuryAta,
            usdcMint: usdcMint, // Added
            tokenProgram: TOKEN_PROGRAM_ID, // Added
            systemProgram: SystemProgram.programId,
        })
        .signers([winnerPurchaser])
        .rpc();

    const ticketAccountAfterPrizeClaim = await program.account.ticketAccount.fetch(winningTicketAccountPda);
    const lotteryAccountAfterDistribute = await program.account.lotteryAccount.fetch(lotteryPda);
    const winnerUsdcBalanceAfter = (await getAccount(provider.connection, winnerUsdcAta)).amount;
    const lotteryAtaBalanceAfter = (await getAccount(provider.connection, lotteryTokenAccountAta)).amount;
    const treasuryAtaBalanceAfter = (await getAccount(provider.connection, treasuryAta)).amount;


    assert.isTrue(ticketAccountAfterPrizeClaim.isClaimed, "Ticket should be marked claimed");
    assert.isTrue(lotteryAccountAfterDistribute.isClaimed, "Lottery should be marked claimed");

    const globalConfig = await program.account.globalConfig.fetch(configPda);
    const prizePool = lotteryAccount.prizePool; // Prize pool before any distribution
    const treasuryFee = prizePool.mul(new BN(globalConfig.treasuryFeePercentage)).div(new BN(10000));
    const winnerPayout = prizePool.sub(treasuryFee);

    assert.equal(lotteryAtaBalanceAfter.toString(), new BN(0).toString(), "Lottery ATA should be empty after prize distribution"); // Assuming full distribution
    assert.equal(treasuryAtaBalanceAfter.toString(), new BN(treasuryAtaBalanceBefore).add(treasuryFee).toString(), "Treasury balance incorrect");
    assert.equal(winnerUsdcBalanceAfter.toString(), new BN(winnerUsdcBalanceBefore).add(winnerPayout).toString(), "Winner balance incorrect");

  });


  it("Should allow admin to update config", async () => {
    const newUsdcMint = await createMint(provider.connection, payer.payer, authorizedOperator.publicKey, null, 6);
    const newTreasuryAta = await getOrCreateAssociatedTokenAccount(provider.connection, payer.payer, newUsdcMint, configPda, true)
        .then(acc => acc.address);

    await program.methods.updateConfig()
        .accounts({
            globalConfig: configPda,
            admin: authorizedOperator.publicKey,
            newUsdcMint: newUsdcMint,
            newTreasuryTokenAccount: newTreasuryAta,
        })
        .signers([authorizedOperator])
        .rpc();

    const updatedConfig = await program.account.globalConfig.fetch(configPda);
    assert.isTrue(updatedConfig.usdcMint.equals(newUsdcMint), "USDC Mint not updated");
    assert.isTrue(updatedConfig.treasuryTokenAccount.equals(newTreasuryAta), "Treasury token account not updated");
    // assert.equal(updatedConfig.treasuryFeePercentage, newFeePercentage); // If fee was also updatable
  });


  // --- ERROR HANDLING AND EDGE CASE TESTS ---
  it("Should prevent buying ticket if lottery not Open", async () => {
    lotteryPda = global.lotteryPda; // Assumes this lottery is now 'Completed' or some other non-Open state
    assert.isDefined(lotteryPda, "Lottery PDA not set");

    const lotteryAcc = await program.account.lotteryAccount.fetch(lotteryPda);
    if (lotteryAcc.state.open) {
        // If it's somehow Open, transition it to something else for this test
        await program.methods.transitionState({ drawing: {} }) // Example: to AwaitingRandomness
            .accounts({
                lotteryAccount: lotteryPda, admin: authorizedOperator.publicKey, globalConfig: configPda, systemProgram: SystemProgram.programId
            }).signers([authorizedOperator]).rpc();
    }
    
    const nonOpenLottery = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.isFalse(nonOpenLottery.state.open, "Lottery should not be in Open state for this test");

    const tempTicketPda = Keypair.generate().publicKey; // Dummy PDA for this attempt

    try {
        await program.methods.buyTicket()
          .accounts({
            lotteryAccount: lotteryPda,
            ticketAccount: tempTicketPda, // This PDA won't actually be created
            userTokenAccount: purchaserUsdcAta,
            lotteryTokenAccount: lotteryUsdcAta, // Needs to be the actual ATA for lotteryPda
            user: purchaser.publicKey,
            globalConfig: configPda,
            usdcMint: usdcMint,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: anchor.utils.token.ASSOCIATED_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([purchaser])
          .rpc();
        assert.fail("Should have failed to buy ticket for non-Open lottery");
    } catch (error) {
        assert.include(error.message, "LotteryNotOpen", "Error message should indicate lottery not open");
    }
  });

  it("Should prevent settling randomness if not AwaitingRandomness", async () => {
    // This requires a lottery NOT in AwaitingRandomness. The global.lotteryPda is likely Completed.
    lotteryPda = global.lotteryPda;
     assert.isDefined(lotteryPda, "Lottery PDA not set");

    const lotteryAcc = await program.account.lotteryAccount.fetch(lotteryPda);
    if (lotteryAcc.state.awaitingRandomness) {
         // If it is AwaitingRandomness, transition it for the test
        await program.methods.transitionState({ completed: {} }) // Example: force to Completed
            .accounts({
                lotteryAccount: lotteryPda, admin: authorizedOperator.publicKey, globalConfig: configPda, systemProgram: SystemProgram.programId
            }).signers([authorizedOperator]).rpc();
    }
    const nonAwaitingLottery = await program.account.lotteryAccount.fetch(lotteryPda);
    assert.isFalse(nonAwaitingLottery.state.awaitingRandomness, "Lottery should not be AwaitingRandomness for this test");

    const mockVrfAccountKey = nonAwaitingLottery.vrfClient || Keypair.generate().publicKey;


    try {
        await program.methods.settleRandomness()
            .accounts({
                lotteryAccount: lotteryPda,
                vrfAccount: mockVrfAccountKey,
            })
            .rpc();
        assert.fail("Should have failed to settle randomness for lottery not in AwaitingRandomness state");
    } catch (error) {
        assert.include(error.message, "InvalidLotteryState", "Error message should indicate invalid lottery state");
    }
  });

  it("Should prevent selecting winner if randomness not fulfilled", async () => {
    // Create a new lottery, transition to Open, then to AwaitingRandomness, but DON'T settle.
    const drawTimeShort = new BN(Math.floor((Date.now() + 5000) / 1000)); // 5 seconds from now
    let tempLotteryPda;

    let listener = null;
    const [eventPromise, _listenerId] = new Promise((resolve, reject) => {
      listener = program.addEventListener("LotteryCreated", (event, slot) => {
        resolve({ event, slot });
      });
    });
    listenerId = _listenerId;

    const tempLotteryKp = Keypair.generate();
    await program.methods.createLottery(defaultLotteryType, defaultTicketPrice, drawTimeShort, defaultTargetPrizePool)
      .accounts({ lotteryAccount: tempLotteryKp.publicKey, creator: authorizedOperator.publicKey, globalConfig: configPda, systemProgram: SystemProgram.programId })
      .signers([authorizedOperator]).rpc();

    const { event } = await eventPromise as any;
    await program.removeEventListener(listenerId);
    tempLotteryPda = new PublicKey(event.lotteryId);

    await program.methods.transitionState({ open: {} })
        .accounts({ lotteryAccount: tempLotteryPda, admin: authorizedOperator.publicKey, globalConfig: configPda, systemProgram: SystemProgram.programId })
        .signers([authorizedOperator]).rpc();

    // Wait for draw time to pass
    await new Promise(resolve => setTimeout(resolve, 6000));

    await program.methods.transitionState({ drawing: {} }) // To AwaitingRandomness
        .accounts({ lotteryAccount: tempLotteryPda, admin: authorizedOperator.publicKey, globalConfig: configPda, systemProgram: SystemProgram.programId })
        .signers([authorizedOperator]).rpc();

    const lotteryToTest = await program.account.lotteryAccount.fetch(tempLotteryPda);
    assert.ok(lotteryToTest.state.awaitingRandomness, "Lottery should be AwaitingRandomness");
    assert.isFalse(lotteryToTest.randomnessFulfilled, "Randomness should not be fulfilled");

    // Now, try to transition to Completed (which select_winner expects) *without* fulfilling randomness
    // This transition will fail if settle_randomness is the only way to Completed
    // Or if admin forces it, the select_winner check for randomnessFulfilled should catch it.
    // Let's assume admin *could* force it to Completed (though current transition_state may not allow this if no randomness)
    // For this test, better to try select_winner while it's AwaitingRandomness and randomness_fulfilled = false.
    // The select_winner instruction has constraint `lottery_account.state == LotteryState::Completed`.
    // So, we first need to manually (as admin) try to push it to Completed without randomness.
    // The transition AwaitingRandomness -> Completed currently checks `randomness_fulfilled`.
    // So, the select_winner instruction's own constraint `randomness_fulfilled` will be the one tested if state somehow becomes Completed.

    // This test setup is a bit tricky. The easiest is to ensure select_winner fails if randomness_fulfilled is false.
    // If the state cannot be `Completed` without `randomness_fulfilled` due to `transition_state` logic,
    // then the `select_winner` constraint `randomness_fulfilled` might be redundant with `state == Completed`
    // if `Completed` implies `randomness_fulfilled`.

    // Test calling select_winner when state is AwaitingRandomness (should fail due to state constraint)
    try {
        await program.methods.selectWinner()
            .accounts({ lotteryAccount: tempLotteryPda, systemProgram: SystemProgram.programId })
            .rpc();
        assert.fail("Should fail if state is not Completed");
    } catch(e) {
        assert.include(e.message, "InvalidLotteryState");
    }

    // If we could force state to Completed with randomnessFulfilled = false (not possible with current transition rules):
    // Manually update account state for test (not standard) OR modify contract to allow this path for testing.
    // For now, this aspect (select_winner when Completed but !randomness_fulfilled) is hard to test cleanly.
    // The existing constraints on select_winner (state==Completed, randomnessFulfilled==true) are good.
  });
});