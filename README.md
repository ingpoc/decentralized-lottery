# Decentralized Lottery

A Solana-based decentralized lottery program.

## Updating USDC Mint Address

We've added a new `update_config` instruction to the program that allows updating the USDC mint address. This is useful when you need to change the token used for the lottery without redeploying the entire program.

### Build and Deploy Steps

1. **Build the program**:
   ```bash
   cd decentralized-lottery
   anchor build
   ```

2. **Deploy the program**:
   ```bash
   anchor deploy
   ```

3. **Update the USDC mint address**:
   ```bash
   # Make sure your Solana CLI is configured with the correct wallet
   solana config get
   
   # Run the update script
   ts-node scripts/update-config.ts
   ```

### Important Notes

- The `update_config` instruction can only be called by the admin wallet that initialized the program.
- Make sure you have the correct permissions and wallet configured before running the update script.
- After updating the USDC mint address, you'll need to update your frontend application to use the new token for buying tickets.

## Frontend Updates

After updating the USDC mint address in the program, update your frontend `.env.local` file:

```
NEXT_PUBLIC_USDC_MINT="Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr"
```

## Troubleshooting

If you encounter any issues:

1. Make sure your wallet has the correct permissions (must be the admin)
2. Ensure you have enough SOL for transaction fees
3. Check that the new USDC mint address is valid
4. Verify that your wallet has a token account for the new USDC mint

## License

This project is licensed under the MIT License - see the LICENSE file for details. 

Program Authority/Owner:
Program Owner: BPFLoaderUpgradeab1e11111111111111111111111 (Solana's BPF Loader)
Program Authority: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Your wallet that deployed it)
Treasury Configuration:
Let me check what treasury accounts are configured in the system:
Perfect! Here are the key IDs for your lottery system:
Owner/Authority IDs:
Program Authority: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Your deployer wallet)
Admin Wallet: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Same as authority - can manage lotteries)
Program Owner: BPFLoaderUpgradeab1e11111111111111111111111 (Solana BPF Loader)
Treasury IDs:
Treasury Wallet: 7Q3UBDfjZgNJNCQBdJrji33f2FvtJ1z3DErcAV6hFsf4 (Defaults to admin wallet)
USDC Mint: Gh9ZwEmdLJ8DscKNTkTqPbNwLNNBjuSzaG9Vp2KGtKJr (Devnet USDC)
System Configuration:
Global Config PDA: Derived from ["global_config"] + program ID
Treasury Fee: 2.5% (250 basis points)
Network: Devnet