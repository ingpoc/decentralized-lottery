"use client";

import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import { useWallet } from "@solana/wallet-adapter-react";
import { useEffect, useState } from "react";
import { PublicKey } from "@solana/web3.js";
import { getAccount, TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync, ASSOCIATED_TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { USDC_MINT, formatUsdc } from "@/lib/constants";
import { connection } from "@/lib/solana/idl";

export function WalletButton() {
  return <WalletMultiButton className="!rounded-lg !bg-indigo-600 hover:!bg-indigo-500 !text-sm" />;
}

/** Shows connected wallet's USDC balance. */
export function UsdcBalance() {
  const { publicKey } = useWallet();
  const [balance, setBalance] = useState<number>(0);

  useEffect(() => {
    if (!USDC_MINT || !publicKey) return;
    const mint = new PublicKey(USDC_MINT);
    const ata = getAssociatedTokenAddressSync(
      mint, publicKey, false, TOKEN_PROGRAM_ID, ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const fetchBal = async () => {
      try {
        const acc = await getAccount(connection, ata);
        setBalance(Number(acc.amount));
      } catch {
        setBalance(0); // No ATA = 0 balance
      }
    };
    fetchBal();
    const interval = setInterval(fetchBal, 10000);
    return () => clearInterval(interval);
  }, [publicKey]);

  return (
    <span className="text-sm font-medium text-gray-600">
      {formatUsdc(balance)} <span className="text-gray-400">USDC</span>
    </span>
  );
}
