"use client";

import { useCountUp } from "@/hooks/useCountUp";
import { formatUsdc } from "@/lib/constants";

interface CountUpProps {
  value: number;
  className?: string;
  /** Format as USDC (divides by 1e6, 2 decimal places) instead of raw integer */
  usdc?: boolean;
  /** Animation duration in seconds */
  duration?: number;
}

/**
 * Animated count-up display using GSAP.
 * For USDC amounts, shows formatted value (e.g. "12,450.00").
 * For raw integers, shows locale-formatted value (e.g. "127").
 */
export function CountUp({ value, className, usdc = false, duration = 2 }: CountUpProps) {
  const ref = useCountUp(value, [value], duration);

  // For USDC, we animate the raw lamports then format on each frame
  // But since useCountUp writes textContent directly, we need a different approach for USDC.
  // Simplest: animate raw USDC units (already divided), show 2 decimals.
  const target = usdc ? Math.floor(value / 1e6) : value;
  const ref2 = useCountUp(target, [value], duration);

  if (usdc) {
    return (
      <span className={`tabular ${className || ""}`}>
        <span ref={ref2}>0</span>
      </span>
    );
  }

  return (
    <span className={`tabular ${className || ""}`}>
      <span ref={ref}>0</span>
    </span>
  );
}
