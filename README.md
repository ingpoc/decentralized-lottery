# Decentralized Lottery

A Solana/Anchor decentralized lottery with USDC prize pools, deterministic randomness, and a Next.js frontend.

## Architecture

- **On-chain program** (`programs/decentralized-lottery/`): Anchor 0.31.1, Solana 2.1.x
- **Frontend** (`frontend/`): Next.js 16 + Tailwind + Phantom wallet adapter
- **Keeper** (`scripts/keeper.ts`): Automated draw lifecycle service

## Lottery lifecycle

```
Created → Open → Locked → Drawing → AwaitingRandomness → Completed
                                                        → Expired (no tickets)
              ↘ Cancelled (admin) ↗
```

1. Admin creates a lottery (type, ticket price, draw time)
2. Lottery opens — users buy tickets with USDC
3. After draw time, keeper locks → draws → settles randomness → selects winner
4. Winner claims prize (2.5% treasury fee deducted)

## Quick start (localnet)

```bash
# Build the program
anchor build

# Run the full lifecycle test
anchor test
```

## Deploy to devnet

```bash
# 1. Fund admin wallet with devnet SOL (need ~3 SOL)
solana airdrop 5 --url devnet  # may be rate-limited

# 2. Deploy the program
anchor deploy --provider.cluster devnet

# 3. Initialize config + create first lottery
npm run setup:devnet

# 4. Update frontend env with the printed values
#    → edit frontend/.env.local

# 5. Run the keeper (draws lotteries automatically)
npm run keeper

# 6. Start the frontend
cd frontend && npm run dev
```

## Program instructions

| Instruction | Who | Description |
|---|---|---|
| `initialize` | Admin | Create global config (fee, USDC mint, treasury) |
| `create_lottery` | Admin | Create a new lottery (daily/weekly/monthly) |
| `transition_state` | Admin/Keeper | Advance lottery through its lifecycle |
| `buy_ticket` | User | Buy a ticket — USDC transferred to lottery vault PDA |
| `settle_randomness` | Anyone | Generate randomness (fallback: on-chain entropy) |
| `select_winner` | Anyone | Determine winning ticket from randomness |
| `claim_prize` | Winner | Claim USDC payout (treasury fee split) |
| `update_config` | Admin | Update fee, admin, or pause state |
| `emergency_pause_toggle` | Admin | Pause/unpause all lottery operations |
| `force_cancel_lottery` | Admin | Emergency cancel a lottery |

## Key PDAs

- GlobalConfig: `["global_config_v2"]`
- Lottery: `["lottery", authority, nonce]`
- Ticket: `["ticket", lottery_key, ticket_id]`
- Vault: ATA of USDC owned by the lottery PDA

## License

MIT
