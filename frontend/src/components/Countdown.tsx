"use client";

import { useCountdown } from "@/hooks/useCountdown";
import clsx from "clsx";

interface CountdownProps {
  targetTime: number;
  className?: string;
}

/**
 * Live ticking countdown display.
 * Shows HH:MM:SS, shifts to danger red when <1h remaining, pulses when urgent.
 */
export function Countdown({ targetTime, className }: CountdownProps) {
  const { display, urgent, expired } = useCountdown(targetTime);

  if (expired) {
    return <span className={clsx("tabular text-[var(--color-muted)]", className)}>{display}</span>;
  }

  return (
    <span
      className={clsx(
        "tabular",
        urgent ? "text-[var(--color-danger)] pulse-dot" : "text-[var(--color-secondary)]",
        className
      )}
    >
      {display}
    </span>
  );
}
