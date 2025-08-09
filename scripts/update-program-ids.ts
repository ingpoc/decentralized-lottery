#!/usr/bin/env ts-node

import * as fs from 'fs';
import * as path from 'path';

const ROOT = path.resolve(__dirname, '..');
const FRONTEND_ROOT = path.resolve(__dirname, '../../crypto-lottery-frontend');
const CONFIG_PATH = path.join(ROOT, 'config/program-ids.json');

type ProgramIds = {
  [network: string]: {
    decentralized_lottery: string;
    decentralized_roulette: string;
  }
}

function loadProgramIds(): ProgramIds {
  return JSON.parse(fs.readFileSync(CONFIG_PATH, 'utf-8'));
}

function updateSourceCode(network: string) {
  const programIds = loadProgramIds();
  const ids = programIds[network];
  
  if (!ids) {
    console.error(`Network ${network} not found in program-ids.json`);
    process.exit(1);
  }

  // Update lottery program source
  const lotteryLibPath = path.join(ROOT, 'programs/decentralized-lottery/src/lib.rs');
  let lotteryContent = fs.readFileSync(lotteryLibPath, 'utf-8');
  lotteryContent = lotteryContent.replace(
    /declare_id!\(".*?"\);/,
    `declare_id!("${ids.decentralized_lottery}");`
  );
  fs.writeFileSync(lotteryLibPath, lotteryContent);
  console.log(`✅ Updated lottery program declare_id!() to ${ids.decentralized_lottery}`);

  // Update roulette program source
  const rouletteLibPath = path.join(ROOT, 'programs/decentralized-roulette/src/lib.rs');
  let rouletteContent = fs.readFileSync(rouletteLibPath, 'utf-8');
  rouletteContent = rouletteContent.replace(
    /declare_id!\(".*?"\);/,
    `declare_id!("${ids.decentralized_roulette}");`
  );
  fs.writeFileSync(rouletteLibPath, rouletteContent);
  console.log(`✅ Updated roulette program declare_id!() to ${ids.decentralized_roulette}`);

  // Update Anchor.toml
  const anchorTomlPath = path.join(ROOT, 'Anchor.toml');
  let anchorContent = fs.readFileSync(anchorTomlPath, 'utf-8');
  
  // Update or add the network section
  const networkSection = `[programs.${network}]\ndecentralized_lottery = "${ids.decentralized_lottery}"\ndecentralized_roulette = "${ids.decentralized_roulette}"`;
  
  if (anchorContent.includes(`[programs.${network}]`)) {
    // Replace existing section - match until next section or end
    const regex = new RegExp(`\\[programs\\.${network}\\][\\s\\S]*?(?=\\n\\[|\\Z)`, 'm');
    anchorContent = anchorContent.replace(regex, networkSection);
  } else {
    // Add new section
    anchorContent += '\n\n' + networkSection;
  }
  
  fs.writeFileSync(anchorTomlPath, anchorContent);
  console.log(`✅ Updated Anchor.toml for ${network}`);
}

function updateIdls(network: string) {
  const programIds = loadProgramIds();
  const ids = programIds[network];
  
  if (!ids) {
    console.error(`Network ${network} not found in program-ids.json`);
    return;
  }

  // Update target IDLs
  const lotteryIdlPath = path.join(ROOT, 'target/idl/decentralized_lottery.json');
  const rouletteIdlPath = path.join(ROOT, 'target/idl/decentralized_roulette.json');
  
  if (fs.existsSync(lotteryIdlPath)) {
    const lotteryIdl = JSON.parse(fs.readFileSync(lotteryIdlPath, 'utf-8'));
    lotteryIdl.address = ids.decentralized_lottery;
    fs.writeFileSync(lotteryIdlPath, JSON.stringify(lotteryIdl, null, 2));
    console.log(`✅ Updated lottery IDL address to ${ids.decentralized_lottery}`);
  }

  if (fs.existsSync(rouletteIdlPath)) {
    const rouletteIdl = JSON.parse(fs.readFileSync(rouletteIdlPath, 'utf-8'));
    rouletteIdl.address = ids.decentralized_roulette;
    fs.writeFileSync(rouletteIdlPath, JSON.stringify(rouletteIdl, null, 2));
    console.log(`✅ Updated roulette IDL address to ${ids.decentralized_roulette}`);
  }

  // Update frontend IDLs
  const frontendLotteryIdl = path.join(FRONTEND_ROOT, 'src/lib/solana/decentralized_lottery.json');
  const frontendRouletteIdl = path.join(FRONTEND_ROOT, 'src/lib/solana/decentralized_roulette.json');
  
  if (fs.existsSync(frontendLotteryIdl)) {
    const idl = JSON.parse(fs.readFileSync(frontendLotteryIdl, 'utf-8'));
    idl.address = ids.decentralized_lottery;
    fs.writeFileSync(frontendLotteryIdl, JSON.stringify(idl, null, 2));
    console.log(`✅ Updated frontend lottery IDL`);
  }

  if (fs.existsSync(frontendRouletteIdl)) {
    const idl = JSON.parse(fs.readFileSync(frontendRouletteIdl, 'utf-8'));
    idl.address = ids.decentralized_roulette;
    fs.writeFileSync(frontendRouletteIdl, JSON.stringify(idl, null, 2));
    console.log(`✅ Updated frontend roulette IDL`);
  }
}

function updateFrontendEnv(network: string, rpcUrl?: string) {
  const programIds = loadProgramIds();
  const ids = programIds[network];
  
  if (!ids) {
    console.error(`Network ${network} not found in program-ids.json`);
    return;
  }

  const defaultRpcUrls: { [key: string]: string } = {
    'localnet': 'http://127.0.0.1:8899',
    'devnet': 'https://api.devnet.solana.com',
    'mainnet': 'https://api.mainnet-beta.solana.com'
  };

  const envRpc = rpcUrl || defaultRpcUrls[network];
  const envPath = path.join(FRONTEND_ROOT, '.env.local');
  
  const envLines = [
    `NEXT_PUBLIC_SOLANA_RPC_URL=${envRpc}`,
    `NEXT_PUBLIC_LOTTERY_PROGRAM_ID=${ids.decentralized_lottery}`,
    `NEXT_PUBLIC_ROULETTE_PROGRAM_ID=${ids.decentralized_roulette}`,
    `# Network: ${network}`,
    `# Updated: ${new Date().toISOString()}`
  ];

  fs.writeFileSync(envPath, envLines.join('\n'));
  console.log(`✅ Updated frontend .env.local for ${network}`);
}

// Main function
function main() {
  const args = process.argv.slice(2);
  const command = args[0];
  const network = args[1];
  
  if (!command || !network) {
    console.log(`
Usage: ts-node scripts/update-program-ids.ts <command> <network> [options]

Commands:
  source       Update source code declare_id!() statements
  idls         Update IDL addresses  
  env          Update frontend environment variables
  all          Update source code, IDLs, and environment

Networks: localnet, devnet, mainnet

Examples:
  ts-node scripts/update-program-ids.ts all localnet
  ts-node scripts/update-program-ids.ts source devnet
  ts-node scripts/update-program-ids.ts env localnet http://127.0.0.1:8899
    `);
    process.exit(1);
  }

  console.log(`🚀 Updating program IDs for ${network}...`);

  switch (command) {
    case 'source':
      updateSourceCode(network);
      break;
    case 'idls':
      updateIdls(network);
      break;
    case 'env':
      const rpcUrl = args[2];
      updateFrontendEnv(network, rpcUrl);
      break;
    case 'all':
      updateSourceCode(network);
      updateIdls(network);
      updateFrontendEnv(network);
      break;
    default:
      console.error(`Unknown command: ${command}`);
      process.exit(1);
  }

  console.log(`✅ Program ID updates completed for ${network}`);
}

if (require.main === module) {
  main();
}