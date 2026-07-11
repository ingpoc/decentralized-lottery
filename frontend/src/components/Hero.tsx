"use client";

import { useEffect, useRef } from "react";
import { gsap } from "gsap";
import { ArrowDown } from "lucide-react";

interface HeroProps {
  totalPool: number;
  activeCount: number;
  totalPlayers: number;
}

export function Hero({ totalPool, activeCount, totalPlayers }: HeroProps) {
  const jackpotRef = useRef<HTMLDivElement>(null);
  const statRef = useRef<HTMLDivElement>(null);
  const ctaRef = useRef<HTMLButtonElement>(null);

  // Animate jackpot number on mount
  useEffect(() => {
    if (!jackpotRef.current) return;

    const targetUsdc = Math.floor(totalPool / 1e6);
    const obj = { val: 0 };
    const tween = gsap.to(obj, {
      val: targetUsdc,
      duration: 2.5,
      ease: "power2.out",
      delay: 0.3,
      onUpdate: () => {
        if (jackpotRef.current) {
          jackpotRef.current.textContent = Math.floor(obj.val).toLocaleString();
        }
      },
    });

    // Stagger entrance
    gsap.from(jackpotRef.current, { y: 30, opacity: 0, duration: 0.8, ease: "power2.out", delay: 0.2 });
    gsap.from(statRef.current, { y: 20, opacity: 0, duration: 0.8, ease: "power2.out", delay: 0.5 });
    gsap.from(ctaRef.current, { y: 20, opacity: 0, duration: 0.8, ease: "power2.out", delay: 0.7 });

    return () => {
      tween.kill();
    };
  }, [totalPool]);

  const scrollToLotteries = () => {
    document.getElementById("lotteries")?.scrollIntoView({ behavior: "smooth", block: "start" });
  };

  return (
    <section className="relative flex min-h-[90vh] flex-col items-center justify-center overflow-hidden px-6">
      {/* Ambient orbs */}
      <div
        className="ambient-orb bg-[var(--color-gold)]"
        style={{ width: 400, height: 400, top: "10%", left: "15%", animation: "float 12s ease-in-out infinite" }}
      />
      <div
        className="ambient-orb bg-[var(--color-green)]"
        style={{ width: 350, height: 350, bottom: "5%", right: "10%", animation: "float 15s ease-in-out infinite reverse" }}
      />
      <div
        className="ambient-orb bg-[var(--color-purple)]"
        style={{ width: 250, height: 250, top: "40%", right: "30%", animation: "float 18s ease-in-out infinite" }}
      />

      {/* Content */}
      <div className="relative z-10 flex flex-col items-center text-center">
        <p className="mb-4 text-xs font-medium uppercase tracking-[0.2em] text-[var(--color-secondary)]">
          Provably Fair · On-Chain · Solana
        </p>

        <div className="mb-2 flex items-center gap-2">
          <span className="text-sm font-medium text-[var(--color-secondary)]">Total Prize Pool</span>
        </div>

        <div className="mb-2 flex items-baseline gap-3">
          <span
            ref={jackpotRef}
            className="tabular text-gradient-gold text-6xl font-bold sm:text-7xl md:text-8xl"
            style={{ letterSpacing: "-0.04em" }}
          >
            0
          </span>
          <span className="text-2xl font-semibold text-[var(--color-gold)] sm:text-3xl">USDC</span>
        </div>

        <div ref={statRef} className="mb-10 flex items-center gap-6 text-sm text-[var(--color-muted)]">
          <span>
            <span className="font-semibold text-[var(--color-secondary)]">{activeCount}</span> active
            {activeCount === 1 ? " draw" : " draws"}
          </span>
          <span className="text-[var(--color-muted)]">·</span>
          <span>
            <span className="font-semibold text-[var(--color-secondary)]">{totalPlayers}</span>{" "}
            {totalPlayers === 1 ? "ticket" : "tickets"} sold
          </span>
          <span className="text-[var(--color-muted)]">·</span>
          <span>drawn on-chain</span>
        </div>

        <button
          ref={ctaRef}
          onClick={scrollToLotteries}
          className="group flex items-center gap-2 rounded-xl bg-[var(--color-green)] px-8 py-3.5 text-sm font-semibold text-black transition-all hover:bg-[var(--color-green-dark)] hover:shadow-[0_0_30px_rgba(20,241,149,0.3)]"
        >
          Play now
          <ArrowDown className="h-4 w-4 transition-transform group-hover:translate-y-0.5" />
        </button>
      </div>
    </section>
  );
}
