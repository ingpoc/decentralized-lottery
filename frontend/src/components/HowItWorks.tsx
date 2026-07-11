"use client";

import { Wallet, Ticket, Trophy } from "lucide-react";

export function HowItWorks() {
  const steps = [
    {
      icon: Wallet,
      title: "Connect",
      desc: "Connect your Phantom wallet on Solana devnet",
    },
    {
      icon: Ticket,
      title: "Buy Tickets",
      desc: "Each ticket is one chance to win the entire pool",
    },
    {
      icon: Trophy,
      title: "Win",
      desc: "Winner takes 97.5% — treasury fee funds the protocol",
    },
  ];

  return (
    <section className="mx-auto max-w-4xl px-6 py-16">
      <div className="grid gap-8 sm:grid-cols-3">
        {steps.map((step, i) => (
          <div key={step.title} className="text-center">
            <div className="mb-4 flex justify-center">
              <div className="flex h-12 w-12 items-center justify-center rounded-xl bg-[var(--color-elevated)] border border-[var(--color-hairline)]">
                <step.icon className="h-5 w-5 text-[var(--color-green)]" />
              </div>
            </div>
            <div className="mb-1 flex items-center justify-center gap-2">
              <span className="text-xs font-medium text-[var(--color-muted)]">{i + 1}</span>
              <h3 className="text-sm font-semibold text-[var(--color-primary)]">{step.title}</h3>
            </div>
            <p className="text-xs leading-relaxed text-[var(--color-secondary)]">{step.desc}</p>
          </div>
        ))}
      </div>
    </section>
  );
}
