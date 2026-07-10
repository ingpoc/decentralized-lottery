/**
 * Devnet configuration — update these after running `npm run setup:devnet`
 * The setup script prints the correct values.
 */

// Program ID (from declare_id! / target/deploy keypair)
export const PROGRAM_ID = process.env.NEXT_PUBLIC_PROGRAM_ID || "H68tD5YsVLrtes5UfAa7iEEvvqcGNj2FGXxQgzMMS4pw";

// Devnet RPC
export const RPC_URL = process.env.NEXT_PUBLIC_RPC_URL || "https://api.devnet.solana.com";

// USDC mint on devnet (created by setup script, or set via env)
export const USDC_MINT = process.env.NEXT_PUBLIC_USDC_MINT || "";

// Cluster
export const CLUSTER = "devnet" as const;

// Lottery type labels
export const LOTTERY_TYPE_LABELS: Record<string, string> = {
  daily: "Daily",
  weekly: "Weekly",
  monthly: "Monthly",
};

// State badge colors
export const STATE_COLORS: Record<string, string> = {
  created: "bg-gray-100 text-gray-700",
  open: "bg-green-100 text-green-700",
  locked: "bg-yellow-100 text-yellow-700",
  drawing: "bg-blue-100 text-blue-700",
  awaitingRandomness: "bg-purple-100 text-purple-700",
  completed: "bg-emerald-100 text-emerald-700",
  expired: "bg-orange-100 text-orange-700",
  cancelled: "bg-red-100 text-red-700",
};

// USDC has 6 decimals
export const USDC_DECIMALS = 6;

export function formatUsdc(amount: number | bigint): string {
  const value = typeof amount === "bigint" ? Number(amount) : amount;
  return (value / Math.pow(10, USDC_DECIMALS)).toLocaleString(undefined, {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export function shortAddress(addr: string | null, chars = 4): string {
  if (!addr) return "";
  return `${addr.slice(0, chars)}...${addr.slice(-chars)}`;
}
