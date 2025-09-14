# Decentralized Lottery

A Solana-based decentralized lottery program with Switchboard VRF integration for provably fair randomness.

## Features

- ✅ **Switchboard VRF Integration**: Production-ready verifiable randomness
- ✅ **Multi-Lottery Support**: Daily, Weekly, Monthly lottery types
- ✅ **USDC Prize Pools**: Stablecoin-based transactions
- ✅ **Emergency Controls**: Admin pause and cancel functionality
- ✅ **Comprehensive Events**: Full audit trail
- ✅ **Fallback Randomness**: Secure fallback for development/testing

## Switchboard VRF Implementation

The lottery program now includes full Switchboard VRF integration for production-ready, provably fair randomness:

### VRF Instructions

1. **Initialize Lottery VRF** - Set up VRF client for a lottery
2. **Request Lottery Randomness** - Request randomness from Switchboard
3. **Consume Lottery Randomness** - Process VRF results and select winner

### Lottery Flow with VRF

1. **Create Lottery** → Admin creates lottery with parameters
2. **Initialize VRF** → Set up Switchboard VRF client
3. **Open for Tickets** → Users can buy tickets
4. **Lock Lottery** → Stop ticket sales
5. **Request Randomness** → Call Switchboard VRF
6. **Consume Randomness** → Process VRF result
7. **Select Winner** → Determine winning ticket
8. **Claim Prize** → Winner claims USDC prize

## Build and Deploy Steps

1. **Build the program**:
   ```bash
   cd decentralized-lottery
   npm run build:full
   ```

2. **Deploy the program**:
   ```bash
   npm run deploy:devnet  # or deploy:mainnet
   ```

3. **Update the USDC mint address** (if needed):
   ```bash
   npm run update-config
   ```

## Important Notes

- The `switchboard-solana` dependency is now enabled in the workspace
- VRF integration requires Switchboard oracle setup and payment
- Fallback randomness is available for development/testing
- All VRF operations require admin authorization

## Troubleshooting

If you encounter any issues:

1. Make sure your wallet has the correct permissions (must be the admin)
2. Ensure you have enough SOL for transaction fees and VRF payments
3. Check that the Switchboard oracle queue is properly configured
4. Verify that your wallet has a token account for the USDC mint

## Program Authority/Owner:
Program Owner: BPFLoaderUpgradeab1e111111111111111111111111 (Solana's BPF Loader)
Program Authority: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Your wallet that deployed it)

Treasury Configuration:
Let me check what treasury accounts are configured in the system:
Perfect! Here are the key IDs for your lottery system:
Owner/Authority IDs:
Program Authority: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Your deployer wallet)
Admin Wallet: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Same as authority - can manage lotteries)
Program Owner: BPFLoaderUpgradeab1e111111111111111111111111 (Solana BPF Loader)

Treasury IDs:
Treasury Wallet: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Defaults to admin wallet)
USDC Mint: Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr (Devnet USDC)

System Configuration:
Global Config PDA: Derived from ["global_config_v2"] + program ID
Treasury Fee: 2.5% (250 basis points)

Network: Devnet

## License

This project is licensed under the MIT License - see the LICENSE file for details.