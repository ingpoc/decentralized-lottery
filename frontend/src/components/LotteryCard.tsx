"use client";

import { useState } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import { LotteryData, useBuyTicket } from "@/hooks/useLottery";
import { formatUsdc, LOTTERY_TYPE_LABELS, STATE_COLORS, shortAddress } from "@/lib/constants";

export function LotteryCard({ lottery }: { lottery: LotteryData }) {
  const { connected } = useWallet();
  const buyTicket = useBuyTicket();
  const [buying, setBuying] = useState(false);
  const [txSig, setTxSig] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const stateLabel = Object.keys({ [lottery.state]: true })[0] || lottery.state;
  const isOpen = lottery.state === "open";
  const drawDate = new Date(lottery.drawTime * 1000);
  const timeUntil = drawDate.getTime() - Date.now();
  const countdown = timeUntil > 0
    ? `${Math.floor(timeUntil / 3600000)}h ${Math.floor((timeUntil % 3600000) / 60000)}m`
    : "Drawn";

  const handleBuy = async () => {
    setBuying(true);
    setError(null);
    setTxSig(null);
    try {
      const result = await buyTicket(lottery);
      setTxSig(result.sig);
    } catch (err: any) {
      setError(err.message || "Failed to buy ticket");
    } finally {
      setBuying(false);
    }
  };

  return (
    <div className="rounded-xl border border-gray-200 bg-white p-5 shadow-sm transition hover:shadow-md">
      <div className="flex items-start justify-between">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">
            {LOTTERY_TYPE_LABELS[lottery.lotteryType] || lottery.lotteryType} Lottery
          </h3>
          <p className="text-xs text-gray-400 mt-0.5">{shortAddress(lottery.publicKey, 6)}</p>
        </div>
        <span className={`rounded-full px-3 py-1 text-xs font-medium capitalize ${STATE_COLORS[lottery.state] || "bg-gray-100"}`}>
          {lottery.state}
        </span>
      </div>

      <div className="mt-4 grid grid-cols-2 gap-3">
        <div>
          <p className="text-xs text-gray-400">Prize Pool</p>
          <p className="text-xl font-bold text-indigo-600">{formatUsdc(lottery.prizePool)} <span className="text-sm">USDC</span></p>
        </div>
        <div>
          <p className="text-xs text-gray-400">Ticket Price</p>
          <p className="text-xl font-bold text-gray-900">{formatUsdc(lottery.ticketPrice)} <span className="text-sm">USDC</span></p>
        </div>
        <div>
          <p className="text-xs text-gray-400">Tickets Sold</p>
          <p className="text-sm font-medium text-gray-700">{lottery.totalTickets}</p>
        </div>
        <div>
          <p className="text-xs text-gray-400">Draws In</p>
          <p className="text-sm font-medium text-gray-700">{countdown}</p>
        </div>
      </div>

      <div className="mt-4">
        {isOpen && (
          <button
            onClick={handleBuy}
            disabled={!connected || buying}
            className="w-full rounded-lg bg-indigo-600 px-4 py-2.5 text-sm font-semibold text-white transition hover:bg-indigo-500 disabled:cursor-not-allowed disabled:bg-gray-300"
          >
            {buying ? "Buying..." : !connected ? "Connect wallet to buy" : "Buy Ticket"}
          </button>
        )}
        {!isOpen && lottery.state === "completed" && lottery.winningTicket && (
          <div className="rounded-lg bg-emerald-50 px-4 py-2 text-center text-sm text-emerald-700">
            🎉 Winner drawn — {shortAddress(lottery.winningTicket, 6)}
          </div>
        )}
        {!isOpen && lottery.state !== "completed" && (
          <div className="rounded-lg bg-gray-50 px-4 py-2 text-center text-sm text-gray-500">
            {lottery.state === "created" ? "Not yet open" : `Lottery ${lottery.state}`}
          </div>
        )}
      </div>

      {txSig && (
        <p className="mt-2 text-xs text-emerald-600">
          ✓ Ticket purchased!{" "}
          <a href={`https://solscan.io/tx/${txSig}?cluster=devnet`} target="_blank" rel="noreferrer" className="underline">
            View tx
          </a>
        </p>
      )}
      {error && <p className="mt-2 text-xs text-red-500">✗ {error}</p>}
    </div>
  );
}
