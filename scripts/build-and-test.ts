#!/usr/bin/env ts-node

import { execSync } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

async function main() {
  console.log('🔨 Building Decentralized Lottery Programs...');

  try {
    // Build lottery program
    console.log('📦 Building lottery program...');
    execSync('cd programs/decentralized-lottery && anchor build', {
      stdio: 'inherit',
      cwd: process.cwd()
    });

    // Build roulette program
    console.log('🎲 Building roulette program...');
    execSync('cd programs/decentralized-roulette && anchor build', {
      stdio: 'inherit',
      cwd: process.cwd()
    });

    // Sync IDLs
    console.log('🔄 Syncing IDLs...');
    execSync('npm run sync-idl', {
      stdio: 'inherit',
      cwd: process.cwd()
    });

    console.log('✅ Build completed successfully!');
    console.log('🎉 Programs are ready for deployment!');

  } catch (error) {
    console.error('❌ Build failed:', error);
    process.exit(1);
  }
}

main().catch(console.error);
