"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { WalletButton, UsdcBalance } from "@/components/WalletButton";
import { LotteryCard } from "@/components/LotteryCard";
import { Hero } from "@/components/Hero";
import { HowItWorks } from "@/components/HowItWorks";
import { useLotteries, LotteryData } from "@/hooks/useLottery";
import { formatUsdc, shortAddress, LOTTERY_TYPE_LABELS } from "@/lib/constants";
import { Trophy, ExternalLink } from "lucide-react";
import Link from "next/link";

export default function Home() {
  const { connected } = useWallet();
  const { lotteries, loading } = useLotteries();

  const activeLotteries = lotteries.filter(
    (l) => l.state === "open" || l.state === "created"
  );
  const pastLotteries = lotteries.filter(
    (l) => l.state === "completed" || l.state === "cancelled" || l.state === "expired"
  );

  // Hero stats
  const totalPool = activeLotteries.reduce((sum, l) => sum + l.prizePool, 0);
  const totalTickets = activeLotteries.reduce((sum, l) => sum + l.totalTickets, 0);

  return (
    <div className="flex-1">
      {/* Header */}
      <header className="sticky top-0 z-50 border-b border-[var(--color-hairline)] bg-[var(--color-base)]/80 backdrop-blur-xl">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-3.5">
          <div className="flex items-center gap-2">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-gold">
              <Trophy className="h-4 w-4 text-black" />
            </div>
            <span className="font-[var(--font-display)] text-sm font-bold text-[var(--color-primary)]">
              Lottery
            </span>
          </div>

          <nav className="hidden items-center gap-1 sm:flex">
            <Link
              href="/"
              className="rounded-lg px-3 py-1.5 text-xs font-medium text-[var(--color-secondary)] transition hover:text-[var(--color-primary)]"
            >
              Play
            </Link>
            <Link
              href="/my-tickets"
              className="rounded-lg px-3 py-1.5 text-xs font-medium text-[var(--color-secondary)] transition hover:text-[var(--color-primary)]"
            >
              My Tickets
            </Link>
            <a
              href="https://github.com/ingpoc/decentralized-lottery"
              target="_blank"
              rel="noreferrer"
              className="rounded-lg px-3 py-1.5 text-xs font-medium text-[var(--color-secondary)] transition hover:text-[var(--color-primary)]"
            >
              GitHub
            </a>
          </nav>

          <div className="flex items-center gap-3">
            {connected && <UsdcBalance />}
            <WalletButton />
          </div>
        </div>
      </header>

      {/* Hero */}
      <Hero totalPool={totalPool} activeCount={activeLotteries.length} totalPlayers={totalTickets} />

      {/* Active Lotteries */}
      <section id="lotteries" className="mx-auto max-w-6xl px-6 py-12">
        <div className="mb-6 flex items-baseline justify-between">
          <h2 className="font-[var(--font-display)] text-lg font-semibold text-[var(--color-primary)]">
            Active Lotteries
          </h2>
          {activeLotteries.length > 0 && (
            <span className="text-xs text-[var(--color-muted)]">
              {activeLotteries.length} {activeLotteries.length === 1 ? "draw" : "draws"} open
            </span>
          )}
        </div>

        {loading ? (
          <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-48 rounded-2xl shimmer" />
            ))}
          </div>
        ) : activeLotteries.length === 0 ? (
          <div className="rounded-2xl border border-[var(--color-hairline)] bg-[var(--color-elevated)] py-20 text-center">
            <p className="text-sm text-[var(--color-secondary)]">No active lotteries right now.</p>
            <p className="mt-1 text-xs text-[var(--color-muted)]">Check back soon.</p>
          </div>
        ) : (
          <div className="grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
            {activeLotteries.map((lottery, i) => (
              <LotteryCard key={lottery.publicKey} lottery={lottery} index={i} />
            ))}
          </div>
        )}
      </section>

      {/* Past Draws — compact table (Tufte: minimal ink for history) */}
      {pastLotteries.length > 0 && (
        <section className="mx-auto max-w-6xl px-6 py-12">
          <h2 className="mb-6 font-[var(--font-display)] text-lg font-semibold text-[var(--color-primary)]">
            Past Draws
          </h2>
          <div className="overflow-hidden rounded-xl border border-[var(--color-hairline)]">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-[var(--color-hairline)] text-xs text-[var(--color-muted)]">
                  <th className="px-4 py-3 text-left font-medium uppercase tracking-wider">Type</th>
                  <th className="px-4 py-3 text-right font-medium uppercase tracking-wider">Pool</th>
                  <th className="px-4 py-3 text-right font-medium uppercase tracking-wider">Tickets</th>
                  <th className="px-4 py-3 text-right font-medium uppercase tracking-wider">Winner</th>
                  <th className="px-4 py-3 text-right font-medium uppercase tracking-wider">State</th>
                </tr>
              </thead>
              <tbody>
                {pastLotteries.slice(0, 10).map((lottery) => (
                  <tr
                    key={lottery.publicKey}
                    className="border-b border-[var(--color-hairline)] last:border-0 transition hover:bg-[var(--color-elevated)]"
                  >
                    <td className="px-4 py-3 text-[var(--color-secondary)]">
                      {LOTTERY_TYPE_LABELS[lottery.lotteryType] || lottery.lotteryType}
                    </td>
                    <td className="px-4 py-3 text-right tabular text-[var(--color-primary)]">
                      {formatUsdc(lottery.prizePool)}
                    </td>
                    <td className="px-4 py-3 text-right tabular text-[var(--color-secondary)]">
                      {lottery.totalTickets}
                    </td>
                    <td className="px-4 py-3 text-right font-[var(--font-mono)] text-xs text-[var(--color-muted)]">
                      {lottery.winningTicket ? shortAddress(lottery.winningTicket, 4) : "—"}
                    </td>
                    <td className="px-4 py-3 text-right">
                      <span className="text-xs capitalize text-[var(--color-muted)]">
                        {lottery.state}
                      </span>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </section>
      )}

      {/* How It Works */}
      <HowItWorks />

      {/* Footer */}
      <footer className="border-t border-[var(--color-hairline)] py-6 text-center">
        <div className="flex items-center justify-center gap-2 text-xs text-[var(--color-muted)]">
          <span>Decentralized Lottery</span>
          <span>·</span>
          <span>Solana Devnet</span>
          <span>·</span>
          <a
            href="https://github.com/ingpoc/decentralized-lottery"
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center gap-1 text-[var(--color-secondary)] transition hover:text-[var(--color-primary)]"
          >
            GitHub <ExternalLink className="h-3 w-3" />
          </a>
        </div>
      </footer>
    </div>
  );
}
