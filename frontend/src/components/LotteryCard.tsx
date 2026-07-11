"use client";

import { useEffect, useRef, useState } from "react";
import { gsap } from "gsap";
import { useWallet } from "@solana/wallet-adapter-react";
import { toast } from "sonner";
import { LotteryData, useBuyTicket } from "@/hooks/useLottery";
import { useClaimPrize } from "@/hooks/useClaimPrize";
import { formatUsdc, LOTTERY_TYPE_LABELS, shortAddress } from "@/lib/constants";
import { Countdown } from "@/components/Countdown";
import { WinnerReveal } from "@/components/WinnerReveal";
import { Ticket, Check, Loader2, ExternalLink, Trophy } from "lucide-react";
import clsx from "clsx";

// Status dot colors
const STATE_DOT: Record<string, string> = {
  created: "bg-[var(--color-muted)]",
  open: "bg-[var(--color-green)] pulse-dot",
  locked: "bg-[var(--color-gold)]",
  drawing: "bg-[var(--color-purple)] pulse-dot",
  awaitingRandomness: "bg-[var(--color-purple)] pulse-dot",
  completed: "bg-[var(--color-gold)]",
  expired: "bg-[var(--color-muted)]",
  cancelled: "bg-[var(--color-danger)]",
};

export function LotteryCard({ lottery, index }: { lottery: LotteryData; index: number }) {
  const { connected, publicKey } = useWallet();
  const buyTicket = useBuyTicket();
  const claimPrize = useClaimPrize();
  const [buying, setBuying] = useState(false);
  const [claiming, setClaiming] = useState(false);
  const [txSig, setTxSig] = useState<string | null>(null);
  const [claimed, setClaimed] = useState(false);
  const cardRef = useRef<HTMLDivElement>(null);
  const barRef = useRef<HTMLDivElement>(null);

  const isOpen = lottery.state === "open";
  const isCompleted = lottery.state === "completed";
  const typeLabel = LOTTERY_TYPE_LABELS[lottery.lotteryType] || lottery.lotteryType;

  // Stagger entrance
  useEffect(() => {
    if (!cardRef.current) return;
    gsap.from(cardRef.current, {
      y: 24,
      opacity: 0,
      duration: 0.5,
      delay: index * 0.08,
      ease: "power2.out",
    });
  }, [index]);

  // Animate progress bar
  useEffect(() => {
    if (!barRef.current) return;
    const pct = Math.min(100, (lottery.totalTickets / 200) * 100); // Assume 200 max for visual
    gsap.to(barRef.current, {
      width: `${pct}%`,
      duration: 1,
      delay: index * 0.08 + 0.3,
      ease: "power2.out",
    });
  }, [lottery.totalTickets, index]);

  const handleBuy = async () => {
    setBuying(true);
    setTxSig(null);
    try {
      const result = await buyTicket(lottery);
      setTxSig(result.sig);
      toast.success("Ticket purchased!", {
        description: `Ticket #${result.ticketId}`,
        action: {
          label: "View",
          onClick: () => window.open(`https://solscan.io/tx/${result.sig}?cluster=devnet`, "_blank"),
        },
      });
    } catch (err: any) {
      toast.error("Purchase failed", { description: err.message?.slice(0, 100) });
    } finally {
      setBuying(false);
    }
  };

  return (
    <div
      ref={cardRef}
      className="card-hover rounded-2xl border border-[var(--color-hairline)] bg-[var(--color-elevated)] p-5"
    >
      {/* Header: type + status */}
      <div className="mb-5 flex items-center justify-between">
        <span className="font-[var(--font-display)] text-xs font-semibold uppercase tracking-wider text-[var(--color-secondary)]">
          {typeLabel} Draw
        </span>
        <div className="flex items-center gap-1.5">
          <span className={clsx("h-1.5 w-1.5 rounded-full", STATE_DOT[lottery.state])} />
          <span className="text-xs capitalize text-[var(--color-muted)]">{lottery.state}</span>
        </div>
      </div>

      {/* Prize pool — the hero number */}
      <div className="mb-1">
        <span className="tabular text-3xl font-bold text-gradient-gold" style={{ letterSpacing: "-0.02em" }}>
          {formatUsdc(lottery.prizePool)}
        </span>
        <span className="ml-1.5 text-sm font-medium text-[var(--color-gold)]">USDC</span>
      </div>
      <p className="mb-5 text-xs uppercase tracking-wider text-[var(--color-muted)]">Prize Pool</p>

      {/* Progress bar — tickets sold (Tufte: data ink) */}
      <div className="mb-1 flex items-center justify-between text-xs">
        <span className="text-[var(--color-muted)]">Tickets sold</span>
        <span className="tabular text-[var(--color-secondary)]">{lottery.totalTickets}</span>
      </div>
      <div className="mb-5 h-1 w-full overflow-hidden rounded-full bg-[var(--color-elevated-2)]">
        <div
          ref={barRef}
          className="h-full rounded-full bg-gradient-green"
          style={{ width: "0%" }}
        />
      </div>

      {/* Inline metadata — entry price + countdown */}
      <div className="mb-5 flex items-center justify-between text-xs">
        <span className="text-[var(--color-secondary)]">
          <span className="tabular font-medium text-[var(--color-primary)]">
            {formatUsdc(lottery.ticketPrice)}
          </span>{" "}
          USDC entry
        </span>
        {isOpen && <Countdown targetTime={lottery.drawTime} />}
      </div>

      {/* Action */}
      {isOpen && (
        <button
          onClick={handleBuy}
          disabled={!connected || buying}
          className={clsx(
            "flex w-full items-center justify-center gap-2 rounded-xl py-3 text-sm font-semibold transition-all",
            !connected || buying
              ? "bg-[var(--color-elevated-2)] text-[var(--color-muted)] cursor-not-allowed"
              : "bg-[var(--color-green)] text-black hover:bg-[var(--color-green-dark)] hover:shadow-[0_0_24px_rgba(20,241,149,0.25)]"
          )}
        >
          {buying ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Buying...
            </>
          ) : !connected ? (
            "Connect wallet"
          ) : txSig ? (
            <>
              <Check className="h-4 w-4" />
              Ticket bought
            </>
          ) : (
            <>
              <Ticket className="h-4 w-4" />
              Buy ticket
            </>
          )}
        </button>
      )}

      {/* Completed — winner reveal animation */}
      {isCompleted && lottery.winningTicket && (
        <>
          <WinnerReveal winningTicket={lottery.winningTicket} />

          {/* Claim button — visible if connected wallet won this lottery */}
          {publicKey && !claimed && (
            <ClaimButton
              lottery={lottery}
  claiming={claiming}
  claimed={claimed}
  onClaim={async () => {
    setClaiming(true);
    try {
      // The winning ticket PDA — we need to derive it from the ticket ID
      // The winningTicket field stores the ticket PDA address
      const sig = await claimPrize(lottery, lottery.winningTicket!);
      setClaimed(true);
      toast.success("Prize claimed!", {
        description: "Your USDC has been transferred",
        action: {
          label: "View tx",
          onClick: () => window.open(`https://solscan.io/tx/${sig}?cluster=devnet`, "_blank"),
        },
      });
    } catch (err: any) {
      toast.error("Claim failed", { description: err.message?.slice(0, 100) });
    } finally {
      setClaiming(false);
    }
  }}
            />
          )}
          {claimed && (
            <div className="mt-2 flex items-center justify-center gap-1.5 rounded-xl bg-[var(--color-green)]/10 py-3 text-sm font-medium text-[var(--color-green)]">
              <Check className="h-4 w-4" /> Prize claimed
            </div>
          )}
        </>
      )}

      {/* Other states */}
      {!isOpen && !isCompleted && (
        <div className="rounded-xl bg-[var(--color-elevated-2)] px-4 py-3 text-center text-sm capitalize text-[var(--color-muted)]">
          {lottery.state === "created" ? "Opens soon" : lottery.state}
        </div>
      )}

      {/* Tx link */}
      {txSig && (
        <a
          href={`https://solscan.io/tx/${txSig}?cluster=devnet`}
          target="_blank"
          rel="noreferrer"
          className="mt-3 flex items-center justify-center gap-1 text-xs text-[var(--color-green)] transition hover:text-[var(--color-green-dark)]"
        >
          View on Solscan <ExternalLink className="h-3 w-3" />
        </a>
      )}
    </div>
  );
}

function ClaimButton({
  claiming,
  claimed,
  onClaim,
}: {
  lottery: LotteryData;
  claiming: boolean;
  claimed: boolean;
  onClaim: () => void;
}) {
  return (
    <button
      onClick={onClaim}
      disabled={claiming || claimed}
      className={clsx(
        "mt-2 flex w-full items-center justify-center gap-2 rounded-xl py-3 text-sm font-semibold transition-all",
        "bg-[var(--color-gold)] text-black hover:shadow-[0_0_24px_rgba(255,176,32,0.25)]",
        (claiming || claimed) && "opacity-60",
      )}
    >
      {claiming ? (
        <>
          <Loader2 className="h-4 w-4 animate-spin" />
          Claiming...
        </>
      ) : (
        <>
          <Trophy className="h-4 w-4" />
          Claim prize
        </>
      )}
    </button>
  );
}
