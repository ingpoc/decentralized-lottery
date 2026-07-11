"use client";

import clsx from "clsx";

export type LotteryFilter = "all" | "daily" | "weekly" | "monthly";

const TABS: { key: LotteryFilter; label: string; dot: string }[] = [
  { key: "all", label: "All", dot: "bg-[var(--color-primary)]" },
  { key: "daily", label: "Daily", dot: "bg-[var(--color-green)]" },
  { key: "weekly", label: "Weekly", dot: "bg-[var(--color-purple)]" },
  { key: "monthly", label: "Monthly", dot: "bg-[var(--color-gold)]" },
];

export function LotteryTabs({
  active,
  onChange,
  counts,
}: {
  active: LotteryFilter;
  onChange: (tab: LotteryFilter) => void;
  counts: Record<string, number>;
}) {
  return (
    <div className="flex items-center gap-1 rounded-xl border border-[var(--color-hairline)] bg-[var(--color-elevated)] p-1">
      {TABS.map((tab) => (
        <button
          key={tab.key}
          onClick={() => onChange(tab.key)}
          className={clsx(
            "flex items-center gap-2 rounded-lg px-4 py-2 text-xs font-medium transition-all",
            active === tab.key
              ? "bg-[var(--color-elevated-2)] text-[var(--color-primary)]"
              : "text-[var(--color-muted)] hover:text-[var(--color-secondary)]"
          )}
        >
          {tab.key !== "all" && (
            <span className={clsx("h-1.5 w-1.5 rounded-full", tab.dot)} />
          )}
          {tab.label}
          {counts[tab.key] > 0 && (
            <span className="tabular text-[10px] text-[var(--color-muted)]">{counts[tab.key]}</span>
          )}
        </button>
      ))}
    </div>
  );
}
