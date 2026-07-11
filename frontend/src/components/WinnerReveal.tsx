"use client";

import { useEffect, useRef, useState } from "react";
import { gsap } from "gsap";
import confetti from "canvas-confetti";
import { shortAddress } from "@/lib/constants";
import { Trophy } from "lucide-react";

interface WinnerRevealProps {
  winningTicket: string;
  onComplete?: () => void;
}

/**
 * Slot-machine style winner reveal animation.
 * Cycles random ticket numbers for 2.5s, then lands on the winner
 * with a scale bounce + gold confetti burst.
 */
export function WinnerReveal({ winningTicket }: WinnerRevealProps) {
  const numberRef = useRef<HTMLSpanElement>(null);
  const [revealed, setRevealed] = useState(false);

  useEffect(() => {
    if (!numberRef.current) return;

    // Phase 1: slot machine — cycle random ticket addresses for 2.5s
    const cycleTl = gsap.timeline();

    cycleTl.to({}, {
      duration: 0.15,
      repeat: 14,
      onRepeat: () => {
        if (numberRef.current) {
          // Generate random-looking address fragments
          const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ123456789abcdefghijkmnopqrstuvwxyz";
          let s = "";
          for (let i = 0; i < 8; i++) s += chars[Math.floor(Math.random() * chars.length)];
          numberRef.current.textContent = s + "...";
        }
      },
    });

    // Phase 2: land on winner
    cycleTl.call(() => {
      if (numberRef.current) {
        numberRef.current.textContent = shortAddress(winningTicket, 6);
      }
      setRevealed(true);
    });

    // Phase 3: scale bounce
    cycleTl.fromTo(
      numberRef.current,
      { scale: 0.5, opacity: 0 },
      { scale: 1, opacity: 1, duration: 0.6, ease: "back.out(2)" },
    );

    // Phase 4: confetti burst
    cycleTl.call(() => {
      const colors = ["#FFB020", "#FF8A00", "#14F195", "#9945FF"];
      // Left side burst
      confetti({
        particleCount: 80,
        spread: 70,
        origin: { x: 0.5, y: 0.6 },
        colors,
        startVelocity: 45,
        scalar: 0.8,
      });
      // Delayed right side burst
      setTimeout(() => {
        confetti({
          particleCount: 50,
          angle: 60,
          spread: 55,
          origin: { x: 0.8, y: 0.7 },
          colors,
          scalar: 0.7,
        });
        confetti({
          particleCount: 50,
          angle: 120,
          spread: 55,
          origin: { x: 0.2, y: 0.7 },
          colors,
          scalar: 0.7,
        });
      }, 300);
    });

    return () => {
      cycleTl.kill();
    };
  }, [winningTicket]);

  return (
    <div className="rounded-xl border border-[var(--color-gold)]/20 bg-[var(--color-elevated-2)] px-4 py-4 text-center">
      <div className="mb-2 flex items-center justify-center gap-1.5">
        <Trophy className={revealed ? "h-3.5 w-3.5 text-[var(--color-gold)]" : "h-3.5 w-3.5 text-[var(--color-muted)]"} />
        <span className="text-xs font-medium uppercase tracking-wider text-[var(--color-gold)]">
          {revealed ? "Winner" : "Drawing..."}
        </span>
      </div>
      <span
        ref={numberRef}
        className="tabular inline-block font-[var(--font-mono)] text-sm font-medium text-[var(--color-gold)]"
        style={{ minHeight: "1.2em", minWidth: "6em" }}
      >
        --------
      </span>
    </div>
  );
}
