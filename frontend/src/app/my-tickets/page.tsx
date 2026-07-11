"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { WalletButton, UsdcBalance } from "@/components/WalletButton";
import { useMyTickets, useLotteries } from "@/hooks/useLottery";
import { formatUsdc, shortAddress, LOTTERY_TYPE_LABELS } from "@/lib/constants";
import { Trophy, ExternalLink, Link as LinkIcon, Ticket as TicketIcon } from "lucide-react";
import Link from "next/link";

export default function MyTicketsPage() {
  const { connected } = useWallet();
  const { tickets, loading } = useMyTickets();
  const { lotteries } = useLotteries();

  // Join tickets with lottery data to determine win status
  const lotteryMap = new Map(lotteries.map((l) => [l.publicKey, l]));

  const enrichedTickets = tickets.map((t: any) => {
    const lottery = lotteryMap.get(t.account.lottery.toBase58());
    const isWinningTicket = lottery?.winningTicket === t.publicKey.toBase58();
    const lotteryCompleted = lottery?.state === "completed";
    return {
      ticketPda: t.publicKey.toBase58(),
      ticketId: t.account.id.toNumber(),
      lotteryPda: t.account.lottery.toBase58(),
      lottery,
      isWinner: isWinningTicket,
      completed: lotteryCompleted,
    };
  });

  return (
    <div className="flex-1">
      <header className="sticky top-0 z-50 border-b border-[var(--color-hairline)] bg-[var(--color-base)]/80 backdrop-blur-xl">
        <div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-3.5">
          <div className="flex items-center gap-4">
            <Link href="/" className="flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-gradient-gold">
                <Trophy className="h-4 w-4 text-black" />
              </div>
              <span className="font-[var(--font-display)] text-sm font-bold">Lottery</span>
            </Link>
            <nav className="flex items-center gap-1">
              <Link href="/" className="rounded-lg px-3 py-1.5 text-xs font-medium text-[var(--color-secondary)] hover:text-[var(--color-primary)]">
                Play
              </Link>
              <span className="rounded-lg bg-[var(--color-elevated)] px-3 py-1.5 text-xs font-medium text-[var(--color-primary)]">
                My Tickets
              </span>
            </nav>
          </div>
          <div className="flex items-center gap-3">
            {connected && <UsdcBalance />}
            <WalletButton />
          </div>
        </div>
      </header>

      <main className="mx-auto max-w-4xl px-6 py-12">
        <h1 className="mb-2 font-[var(--font-display)] text-2xl font-bold text-[var(--color-primary)]">
          My Tickets
        </h1>
        <p className="mb-8 text-sm text-[var(--color-secondary)]">
          {connected
            ? `${enrichedTickets.length} ticket${enrichedTickets.length === 1 ? "" : "s"}`
            : "Connect your wallet to view your tickets"}
        </p>

        {!connected ? (
          <div className="rounded-2xl border border-[var(--color-hairline)] bg-[var(--color-elevated)] py-20 text-center">
            <p className="text-sm text-[var(--color-secondary)]">No wallet connected</p>
          </div>
        ) : loading ? (
          <div className="space-y-3">
            {[1, 2, 3].map((i) => (
              <div key={i} className="h-16 rounded-xl shimmer" />
            ))}
          </div>
        ) : enrichedTickets.length === 0 ? (
          <div className="rounded-2xl border border-[var(--color-hairline)] bg-[var(--color-elevated)] py-20 text-center">
            <TicketIcon className="mx-auto mb-3 h-8 w-8 text-[var(--color-muted)]" />
            <p className="text-sm text-[var(--color-secondary)]">No tickets yet.</p>
            <Link
              href="/"
              className="mt-4 inline-block text-sm font-medium text-[var(--color-green)] hover:text-[var(--color-green-dark)]"
            >
              Browse lotteries →
            </Link>
          </div>
        ) : (
          <div className="space-y-3">
            {enrichedTickets.map((t) => (
              <div
                key={t.ticketPda}
                className="flex items-center justify-between rounded-xl border border-[var(--color-hairline)] bg-[var(--color-elevated)] px-5 py-4"
              >
                <div className="flex items-center gap-4">
                  <div
                    className={`flex h-10 w-10 items-center justify-center rounded-lg ${
                      t.isWinner
                        ? "bg-[var(--color-gold)]/15"
                        : "bg-[var(--color-elevated-2)]"
                    }`}
                  >
                    {t.isWinner ? (
                      <Trophy className="h-5 w-5 text-[var(--color-gold)]" />
                    ) : (
                      <TicketIcon className="h-5 w-5 text-[var(--color-muted)]" />
                    )}
                  </div>
                  <div>
                    <p className="text-sm font-medium text-[var(--color-primary)]">
                      Ticket #{t.ticketId}
                    </p>
                    <p className="text-xs text-[var(--color-muted)]">
                      {t.lottery
                        ? `${LOTTERY_TYPE_LABELS[t.lottery.lotteryType] || t.lottery.lotteryType} Draw · ${formatUsdc(t.lottery.prizePool)} USDC pool`
                        : shortAddress(t.lotteryPda, 6)}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  {t.isWinner && (
                    <span className="rounded-full bg-[var(--color-gold)]/15 px-3 py-1 text-xs font-medium text-[var(--color-gold)]">
                      Winner!
                    </span>
                  )}
                  {t.completed && !t.isWinner && (
                    <span className="text-xs text-[var(--color-muted)]">Not drawn</span>
                  )}
                  {!t.completed && t.lottery && (
                    <span className="text-xs capitalize text-[var(--color-muted)]">{t.lottery.state}</span>
                  )}
                  <a
                    href={`https://solscan.io/account/${t.ticketPda}?cluster=devnet`}
                    target="_blank"
                    rel="noreferrer"
                    className="text-[var(--color-muted)] transition hover:text-[var(--color-secondary)]"
                  >
                    <ExternalLink className="h-3.5 w-3.5" />
                  </a>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}
