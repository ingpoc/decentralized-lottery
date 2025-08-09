#!/usr/bin/env ts-node

import { Connection, Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL } from '@solana/web3.js';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { spawn } from 'child_process';
import * as anchor from '@coral-xyz/anchor';
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from '@solana/spl-token';

const ROOT = path.resolve(__dirname, '..');
const REPO_ROOT = path.resolve(__dirname, '../');
const FRONTEND_ROOT = path.resolve(__dirname, '../../crypto-lottery-frontend');

const DEFAULT_RPC = process.env.RPC_URL || 'http://127.0.0.1:8899';
const DEFAULT_KEYPAIR = process.env.ANCHOR_WALLET || path.join(os.homedir(), '.config/solana/id.json');
const TOOL_PATHS = [
  path.join(os.homedir(), '.cargo/bin'),
  path.join(os.homedir(), '.local/share/solana/install/active_release/bin'),
].join(':');

function log(msg: string) {
  console.log(`[local-deploy] ${msg}`);
}

function withToolEnv(env: NodeJS.ProcessEnv = process.env): NodeJS.ProcessEnv {
  const merged = { ...env };
  const existingPath = env.PATH || env.Path || env.path || '';
  merged.PATH = `${TOOL_PATHS}:${existingPath}`;
  merged.ANCHOR_WALLET = merged.ANCHOR_WALLET || DEFAULT_KEYPAIR;
  return merged;
}

function run(cmd: string, args: string[], cwd = REPO_ROOT, env: NodeJS.ProcessEnv = process.env) {
  return new Promise<{ code: number; stdout: string; stderr: string }>((resolve) => {
    const child = spawn(cmd, args, { cwd, env: withToolEnv(env), stdio: ['ignore', 'pipe', 'pipe'] });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', d => (stdout += d.toString()));
    child.stderr.on('data', d => (stderr += d.toString()));
    child.on('close', code => resolve({ code: code ?? 0, stdout, stderr }));
  });
}

async function ensureAirdrop(connection: Connection, pubkey: PublicKey, minSol = 2) {
  const balance = await connection.getBalance(pubkey);
  if (balance < minSol * LAMPORTS_PER_SOL) {
    log(`Airdropping ${(minSol * LAMPORTS_PER_SOL - balance) / LAMPORTS_PER_SOL} SOL...`);
    await connection.requestAirdrop(pubkey, minSol * LAMPORTS_PER_SOL);
    // wait a bit
    await new Promise(r => setTimeout(r, 1500));
  }
}

async function main() {
  log(`RPC: ${DEFAULT_RPC}`);
  log(`Wallet: ${DEFAULT_KEYPAIR}`);

  // Load admin keypair
  const secret = JSON.parse(fs.readFileSync(DEFAULT_KEYPAIR, 'utf-8')) as number[];
  const admin = Keypair.fromSecretKey(Uint8Array.from(secret));
  const connection = new Connection(DEFAULT_RPC, 'confirmed');
  await ensureAirdrop(connection, admin.publicKey, 5);

  // Build and deploy programs to localnet
  if (!process.env.SKIP_BUILD) {
    log('Building Anchor workspace...');
    const resBuild = await run('anchor', ['build'], REPO_ROOT, { ...process.env });
    if (resBuild.code !== 0) {
      console.error(resBuild.stderr);
      throw new Error('anchor build failed');
    }
  } else {
    log('Skipping build (SKIP_BUILD=1)');
  }

  if (!process.env.SKIP_DEPLOY) {
    log('Deploying to localnet...');
    const resDeploy = await run('anchor', ['deploy', '--provider.cluster', 'localnet'], REPO_ROOT, { ...process.env });
    if (resDeploy.code !== 0) {
      console.error(resDeploy.stderr);
      throw new Error('anchor deploy failed');
    }
  } else {
    log('Skipping deploy (SKIP_DEPLOY=1)');
  }

  // Load IDLs from target/idl
  const lotteryIdlPath = path.join(REPO_ROOT, 'target/idl/decentralized_lottery.json');
  const rouletteIdlPath = path.join(REPO_ROOT, 'target/idl/decentralized_roulette.json');
  const lotteryIdl = JSON.parse(fs.readFileSync(lotteryIdlPath, 'utf-8'));
  const rouletteIdl = JSON.parse(fs.readFileSync(rouletteIdlPath, 'utf-8'));
  const lotteryProgramId = new PublicKey(lotteryIdl.address);
  const rouletteProgramId = new PublicKey(rouletteIdl.address);

  log(`Lottery Program: ${lotteryProgramId.toBase58()}`);
  log(`Roulette Program: ${rouletteProgramId.toBase58()}`);

  // Create or use USDC mint
  let usdcMintKey: PublicKey;
  if (process.env.USDC_MINT) {
    usdcMintKey = new PublicKey(process.env.USDC_MINT);
    log(`Using existing USDC mint: ${usdcMintKey.toBase58()}`);
  } else {
    log('Creating local USDC mint (6 decimals)...');
    usdcMintKey = await createMint(connection, admin, admin.publicKey, null, 6);
    log(`Created USDC mint: ${usdcMintKey.toBase58()}`);
  }

  // Create admin ATA and mint tokens
  const adminUsdcAta = await getOrCreateAssociatedTokenAccount(connection, admin, usdcMintKey, admin.publicKey);
  if ((await connection.getTokenAccountBalance(adminUsdcAta.address)).value.uiAmount ?? 0 < 10000) {
    log('Minting 1,000,000 USDC to admin (for testing)...');
    await mintTo(connection, admin, usdcMintKey, adminUsdcAta.address, admin, 1_000_000_000_000); // 1,000,000 USDC with 6 decimals
  }
  log(`Admin USDC ATA: ${adminUsdcAta.address.toBase58()}`);

  // Initialize programs via Anchor
  const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(admin), { commitment: 'confirmed' });
  anchor.setProvider(provider);

  // Initialize Lottery
  if (!process.env.SKIP_INIT) {
    const program = new anchor.Program(lotteryIdl as any, provider);
    const [globalConfig] = PublicKey.findProgramAddressSync([Buffer.from('global_config_v2')], lotteryProgramId);
    const account = await connection.getAccountInfo(globalConfig);
    if (!account) {
      log('Initializing Lottery global config...');
      await (program.methods as any)
        .initialize()
        .accounts({
          globalConfig,
          admin: admin.publicKey,
          usdcMint: usdcMintKey,
          treasuryTokenAccount: adminUsdcAta.address,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      log('Lottery initialized');
    } else {
      log('Lottery already initialized');
    }
  } else {
    log('Skipping initialization (SKIP_INIT=1)');
  }

  // Initialize Roulette
  if (!process.env.SKIP_INIT) {
    const program = new anchor.Program(rouletteIdl as any, provider);
    const [globalConfig] = PublicKey.findProgramAddressSync([Buffer.from('global_config_v2')], rouletteProgramId);
    const gcAccount = await connection.getAccountInfo(globalConfig);
    if (!gcAccount) {
      log('Initializing Roulette global config (and first roulette)...');
      const [firstRoulette] = PublicKey.findProgramAddressSync(
        [Buffer.from('roulette'), admin.publicKey.toBuffer(), Buffer.from([1, 0, 0, 0, 0, 0, 0, 0])],
        rouletteProgramId
      );
      const [firstRouletteToken] = PublicKey.findProgramAddressSync(
        [Buffer.from('roulette_token'), firstRoulette.toBuffer()],
        rouletteProgramId
      );
      await (program.methods as any)
        .initialize()
        .accounts({
          globalConfig,
          firstRoulette,
          authority: admin.publicKey,
          usdcMint: usdcMintKey,
          treasuryTokenAccount: adminUsdcAta.address,
          firstRouletteTokenAccount: firstRouletteToken,
          tokenProgram: new PublicKey('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
          systemProgram: SystemProgram.programId,
          rent: new PublicKey('SysvarRent111111111111111111111111111111111'),
        })
        .rpc();
      log('Roulette initialized');
    } else {
      log('Roulette already initialized');
    }
  }

  // Sync IDLs to frontend
  if (!process.env.SKIP_IDL_SYNC) {
    log('Syncing IDLs to frontend...');
    const syncRes = await run('npm', ['run', 'sync-idl'], REPO_ROOT, process.env);
    if (syncRes.code !== 0) {
      console.error(syncRes.stderr);
      throw new Error('IDL sync failed');
    }
  } else {
    log('Skipping IDL sync (SKIP_IDL_SYNC=1)');
  }

  // Write/update frontend .env.local helper (non-destructive: append hints)
  try {
    const envPath = path.join(FRONTEND_ROOT, '.env.local');
    const lines = [
      `NEXT_PUBLIC_SOLANA_RPC_URL=${DEFAULT_RPC}`,
      `NEXT_PUBLIC_LOTTERY_PROGRAM_ID=${lotteryProgramId.toBase58()}`,
      `NEXT_PUBLIC_ROULETTE_PROGRAM_ID=${rouletteProgramId.toBase58()}`,
      `NEXT_PUBLIC_USDC_MINT=${usdcMintKey.toBase58()}`,
      `NEXT_PUBLIC_ADMIN_WALLET=${admin.publicKey.toBase58()}`,
      `NEXT_PUBLIC_TREASURY_WALLET=${admin.publicKey.toBase58()}`,
    ];
    fs.writeFileSync(envPath, lines.join('\n'));
    log(`Wrote frontend env: ${envPath}`);
  } catch (e) {
    log(`Skipped writing frontend .env.local: ${e}`);
  }

  log('All done. Frontend can now run against local validator.');
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
