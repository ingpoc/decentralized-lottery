#!/bin/bash

# Exit on error
set -e

# Print commands before executing
set -x

echo "🧹 Cleaning previous build artifacts..."
/Users/gurusharan/.cargo/bin/anchor clean

echo "🔨 Building Anchor program..."
/Users/gurusharan/.cargo/bin/anchor build

echo "📝 Updating IDL and TypeScript types..."
npm run update-idl

echo "✅ Build process completed successfully!"
echo ""
echo "You can now:"
echo "  - Deploy the program: anchor deploy"
echo "  - Run tests: anchor test"
echo "  - Update USDC mint: npm run update-config"
echo "" 