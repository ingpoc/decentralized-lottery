"use client";

import idlJson from "./decentralized_lottery.json";
import { PROGRAM_ID, RPC_URL } from "@/lib/constants";
import { AnchorProvider, Program, setProvider, Idl } from "@coral-xyz/anchor";
import { Connection, PublicKey } from "@solana/web3.js";
import type { AnchorWallet } from "@solana/wallet-adapter-react";

export const IDL = idlJson as unknown as Idl;

export const connection = new Connection(RPC_URL, "confirmed");

export const programId = new PublicKey(PROGRAM_ID);

/** Create a program instance bound to a wallet (for signed transactions). */
export function getProgram(wallet: AnchorWallet): Program {
  const provider = new AnchorProvider(connection, wallet, {
    preflightCommitment: "confirmed",
  });
  setProvider(provider);
  return new Program(IDL, provider);
}

/** Read-only program instance (no wallet needed). */
export function getReadonlyProgram(): Program {
  const provider = new AnchorProvider(
    connection,
    // Dummy wallet for reads — never signs
    {
      publicKey: PublicKey.default,
      signTransaction: async () => {
        throw new Error("Read-only program cannot sign");
      },
      signAllTransactions: async () => {
        throw new Error("Read-only program cannot sign");
      },
    } as any,
    { preflightCommitment: "confirmed" }
  );
  return new Program(IDL, provider);
}
