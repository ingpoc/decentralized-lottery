"use client";

import { useCallback, useEffect, useState } from "react";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey, SystemProgram, SYSVAR_SLOT_HASHES_PUBKEY, SYSVAR_CLOCK_PUBKEY } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { BN, Program } from "@coral-xyz/anchor";
import { getProgram, getReadonlyProgram, programId, connection } from "@/lib/solana/idl";
import { USDC_MINT } from "@/lib/constants";

export interface LotteryData {
  publicKey: string;
  lotteryType: string;
  ticketPrice: number;
  drawTime: number;
  prizePool: number;
  totalTickets: number;
  state: string;
  authority: string;
  winningTicket: string | null;
  nonce: number;
}

/** Fetch all lottery accounts. */
export function useLotteries() {
  const [lotteries, setLotteries] = useState<LotteryData[]>([]);
  const [loading, setLoading] = useState(true);

  const refresh = useCallback(async () => {
    try {
      const program = getReadonlyProgram();
      const accounts = await (program.account as any).lotteryAccount.all();
      const mapped: LotteryData[] = accounts.map((acc: any) => ({
        publicKey: acc.publicKey.toBase58(),
        lotteryType: Object.keys(acc.account.lotteryType)[0],
        ticketPrice: acc.account.ticketPrice.toNumber(),
        drawTime: acc.account.drawTime,
        prizePool: acc.account.prizePool.toNumber(),
        totalTickets: acc.account.totalTickets,
        state: Object.keys(acc.account.state)[0],
        authority: acc.account.authority.toBase58(),
        winningTicket: acc.account.winningTicket?.toBase58() ?? null,
        nonce: acc.account.nonce.toNumber(),
      }));
      mapped.sort((a, b) => b.drawTime - a.drawTime);
      setLotteries(mapped);
    } catch (err) {
      console.error("Failed to fetch lotteries:", err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    // Refresh every 15 seconds
    const interval = setInterval(refresh, 15000);
    return () => clearInterval(interval);
  }, [refresh]);

  return { lotteries, loading, refresh };
}

/** Derive the global config PDA. */
export function getGlobalConfigPda(): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("global_config_v2")],
    programId
  )[0];
}

/** Derive a lottery PDA from authority + nonce. */
export function getLotteryPda(authority: PublicKey, nonce: BN): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("lottery"), authority.toBuffer(), nonce.toArrayLike(Buffer, "le", 8)],
    programId
  )[0];
}

/** Derive a ticket PDA. */
export function getTicketPda(lotteryKey: PublicKey, ticketId: number): PublicKey {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("ticket"), lotteryKey.toBuffer(), new BN(ticketId).toArrayLike(Buffer, "le", 8)],
    programId
  )[0];
}

/** Buy a ticket for a lottery. */
export function useBuyTicket() {
  const wallet = useAnchorWallet();
  const { connection } = useConnection();

  return useCallback(
    async (lottery: LotteryData) => {
      if (!wallet) throw new Error("Wallet not connected");
      if (!USDC_MINT) throw new Error("USDC_MINT not configured — run setup:devnet first");

      const program = getProgram(wallet);
      const usdcMint = new PublicKey(USDC_MINT);
      const lotteryKey = new PublicKey(lottery.publicKey);
      const buyer = wallet.publicKey;

      // Ticket PDA — ID is last_ticket_id + 1 (we don't know last_ticket_id, so fetch it)
      const lotteryAccount = await (program.account as any).lotteryAccount.fetch(lotteryKey);
      const ticketId = lotteryAccount.lastTicketId.toNumber() + 1;
      const ticketPda = getTicketPda(lotteryKey, ticketId);

      // Buyer USDC ATA
      const buyerAta = getAssociatedTokenAddressSync(
        usdcMint, buyer, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
      );
      // Create buyer ATA if it doesn't exist
      await getOrCreateAssociatedTokenAccount(connection, wallet as any, usdcMint, buyer);

      // Lottery vault ATA (owned by lottery PDA)
      const vaultAta = getAssociatedTokenAddressSync(
        usdcMint, lotteryKey, true, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
      );

      const globalConfigPda = getGlobalConfigPda();

      const sig = await (program.methods as any)
        .buyTicket()
        .accounts({
          lotteryAccount: lotteryKey,
          ticketAccount: ticketPda,
          globalConfig: globalConfigPda,
          user: buyer,
          userTokenAccount: buyerAta,
          lotteryTokenAccount: vaultAta,
          usdcMint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      return { sig, ticketPda: ticketPda.toBase58(), ticketId };
    },
    [wallet, connection]
  );
}

/** Fetch tickets owned by the connected wallet. */
export function useMyTickets() {
  const wallet = useAnchorWallet();
  const [tickets, setTickets] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!wallet) return;
    setLoading(true);
    try {
      const program = getReadonlyProgram();
      const allTickets = await (program.account as any).ticketAccount.all([
        { memcmp: { offset: 8 + 32, bytes: wallet.publicKey.toBase58() } },
      ]);
      setTickets(allTickets);
    } catch (err) {
      console.error("Failed to fetch tickets:", err);
    } finally {
      setLoading(false);
    }
  }, [wallet]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { tickets, loading, refresh };
}
