"use client";

import { useCallback } from "react";
import { useAnchorWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import { getProgram, programId } from "@/lib/solana/idl";
import { USDC_MINT } from "@/lib/constants";
import { LotteryData, getGlobalConfigPda } from "@/hooks/useLottery";

/**
 * Hook to claim a lottery prize for a winning ticket.
 * Returns a function that takes the lottery data + the winning ticket ID.
 */
export function useClaimPrize() {
  const wallet = useAnchorWallet();

  return useCallback(
    async (lottery: LotteryData, ticketPda: string) => {
      if (!wallet) throw new Error("Wallet not connected");
      if (!USDC_MINT) throw new Error("USDC_MINT not configured");

      const program = getProgram(wallet);
      const usdcMint = new PublicKey(USDC_MINT);
      const lotteryKey = new PublicKey(lottery.publicKey);
      const winner = wallet.publicKey;
      const ticketKey = new PublicKey(ticketPda);

      // Derive all required accounts
      const globalConfigPda = getGlobalConfigPda();

      // Winner USDC ATA
      const winnerAta = getAssociatedTokenAddressSync(
        usdcMint, winner, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
      );

      // Lottery vault ATA (owned by lottery PDA)
      const vaultAta = getAssociatedTokenAddressSync(
        usdcMint, lotteryKey, true, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
      );

      // Treasury ATA — need to fetch config to get it
      const config = await (program.account as any).globalConfig.fetch(globalConfigPda);
      const treasuryAta = config.treasuryTokenAccount;

      const sig = await (program.methods as any)
        .claimPrize()
        .accounts({
          lotteryAccount: lotteryKey,
          ticketAccount: ticketKey,
          globalConfig: globalConfigPda,
          winner: winner,
          lotteryTokenAccount: vaultAta,
          winnerTokenAccount: winnerAta,
          treasuryTokenAccount: treasuryAta,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();

      return sig;
    },
    [wallet]
  );
}
