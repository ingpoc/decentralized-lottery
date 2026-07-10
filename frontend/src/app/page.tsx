"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { WalletButton, UsdcBalance } from "@/components/WalletButton";
import { LotteryCard } from "@/components/LotteryCard";
import { useLotteries } from "@/hooks/useLottery";

export default function Home() {
  const { connected } = useWallet();
  const { lotteries, loading } = useLotteries();

  const activeLotteries = lotteries.filter(
    (l) => l.state === "open" || l.state === "created"
  );
  const pastLotteries = lotteries.filter(
    (l) => l.state === "completed" || l.state === "cancelled" || l.state === "expired"
  );

  return (
    <div className="flex-1">
      {/* Header */}
      <header className="border-b border-gray-200 bg-white">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
          <div className="flex items-center gap-2">
            <span className="text-2xl">🎟️</span>
            <div>
              <h1 className="text-lg font-bold text-gray-900">Decentralized Lottery</h1>
              <p className="text-xs text-gray-400">Solana · USDC · Provably Fair</p>
            </div>
          </div>
          <div className="flex items-center gap-4">
            {connected && <UsdcBalance />}
            <WalletButton />
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-6 py-8">
        {/* Active Lotteries */}
        <section>
          <h2 className="mb-4 text-sm font-semibold uppercase tracking-wide text-gray-500">
            Active Lotteries
          </h2>
          {loading ? (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {[1, 2, 3].map((i) => (
                <div key={i} className="h-56 animate-pulse rounded-xl bg-gray-200" />
              ))}
            </div>
          ) : activeLotteries.length === 0 ? (
            <div className="rounded-xl border border-dashed border-gray-300 bg-white py-16 text-center">
              <p className="text-gray-400">No active lotteries right now.</p>
              <p className="mt-1 text-xs text-gray-400">Check back soon or ask the admin to create one.</p>
            </div>
          ) : (
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {activeLotteries.map((lottery) => (
                <LotteryCard key={lottery.publicKey} lottery={lottery} />
              ))}
            </div>
          )}
        </section>

        {/* Past Lotteries */}
        {pastLotteries.length > 0 && (
          <section className="mt-10">
            <h2 className="mb-4 text-sm font-semibold uppercase tracking-wide text-gray-500">
              Past Draws
            </h2>
            <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
              {pastLotteries.slice(0, 6).map((lottery) => (
                <LotteryCard key={lottery.publicKey} lottery={lottery} />
              ))}
            </div>
          </section>
        )}

        {/* Info section */}
        <section className="mt-12 rounded-xl bg-gradient-to-br from-indigo-600 to-purple-600 p-8 text-white">
          <h3 className="text-lg font-semibold">How it works</h3>
          <div className="mt-4 grid gap-6 sm:grid-cols-3">
            <div>
              <p className="text-2xl">1️⃣</p>
              <p className="mt-1 text-sm text-indigo-100">Connect your Phantom wallet on Solana devnet</p>
            </div>
            <div>
              <p className="text-2xl">2️⃣</p>
              <p className="mt-1 text-sm text-indigo-100">Buy tickets with USDC — every ticket is a chance to win</p>
            </div>
            <div>
              <p className="text-2xl">3️⃣</p>
              <p className="mt-1 text-sm text-indigo-100">Winner is selected on-chain via deterministic randomness</p>
            </div>
          </div>
        </section>
      </main>

      <footer className="border-t border-gray-200 py-6 text-center text-xs text-gray-400">
        Decentralized Lottery · Solana Devnet ·{" "}
        <a href="https://github.com/ingpoc/decentralized-lottery" target="_blank" rel="noreferrer" className="underline">
          GitHub
        </a>
      </footer>
    </div>
  );
}
