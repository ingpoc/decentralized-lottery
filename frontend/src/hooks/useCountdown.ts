"use client";

import { useEffect, useState } from "react";

/**
 * Live ticking countdown to a target timestamp.
 * Updates every second, returns formatted HH:MM:SS and whether time is "urgent" (<1h).
 *
 * @param targetTime Unix timestamp (seconds) to count down to
 * @returns { display: "HH:MM:SS" | "Drawn", urgent: boolean, expired: boolean }
 */
export function useCountdown(targetTime: number) {
  const [now, setNow] = useState(() => Math.floor(Date.now() / 1000));

  useEffect(() => {
    const interval = setInterval(() => {
      setNow(Math.floor(Date.now() / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, []);

  const diff = targetTime - now;

  if (diff <= 0) {
    return { display: "Drawn", urgent: false, expired: true };
  }

  const hours = Math.floor(diff / 3600);
  const minutes = Math.floor((diff % 3600) / 60);
  const seconds = diff % 60;

  let display: string;
  if (hours > 0) {
    display = `${hours}:${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
  } else {
    display = `${minutes}:${String(seconds).padStart(2, "0")}`;
  }

  return { display, urgent: diff < 3600, expired: false };
}
