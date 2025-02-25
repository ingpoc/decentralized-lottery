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