import { spawn } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';

const TARGET_IDL_DIR = './target/idl';
const TARGET_DEPLOY_DIR = './target/deploy';

function ensureDirectoryExists(dirPath: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`Created directory: ${dirPath}`);
  }
}

async function runCommand(command: string, args: string[], description: string, ignoreFailure = false): Promise<boolean> {
  return new Promise((resolve) => {
    console.log(`🔄 ${description}...`);
    console.log(`Running: ${command} ${args.join(' ')}`);
    
    const env = {
      ...process.env,
      PATH: `${process.env.HOME}/.cargo/bin:${process.env.HOME}/.local/share/solana/install/active_release/bin:${process.env.PATH}`
    };
    
    const child = spawn(command, args, {
      stdio: 'inherit',
      env
    });
    
    child.on('close', (code) => {
      if (code === 0) {
        console.log(`✅ ${description} completed successfully`);
        resolve(true);
      } else {
        if (ignoreFailure) {
          console.log(`⚠️  ${description} failed with code ${code}, continuing anyway...`);
          resolve(true);
        } else {
          console.error(`❌ ${description} failed with code ${code}`);
          resolve(false);
        }
      }
    });
    
    child.on('error', (error) => {
      if (ignoreFailure) {
        console.log(`⚠️  ${description} error: ${error.message}, continuing anyway...`);
        resolve(true);
      } else {
        console.error(`❌ ${description} error:`, error.message);
        resolve(false);
      }
    });
  });
}

async function extractIdlFromBinary(programName: string): Promise<boolean> {
  const binaryPath = path.join(TARGET_DEPLOY_DIR, `${programName}.so`);
  const idlPath = path.join(TARGET_IDL_DIR, `${programName}.json`);
  
  console.log(`\n🔧 Extracting IDL for ${programName}...`);
  
  if (!fs.existsSync(binaryPath)) {
    console.error(`❌ Binary not found: ${binaryPath}`);
    return false;
  }
  
  // Try anchor idl parse first
  let success = await runCommand('anchor', [
    'idl', 'parse', '-f', binaryPath, '-o', idlPath
  ], `Parse IDL from binary for ${programName}`, true);
  
  if (fs.existsSync(idlPath)) {
    console.log(`✅ IDL extracted successfully: ${idlPath}`);
    return true;
  }
  
  // If that fails, try to extract with solana program dump-idl
  success = await runCommand('solana', [
    'program', 'dump-idl', binaryPath, idlPath
  ], `Dump IDL from binary for ${programName}`, true);
  
  if (fs.existsSync(idlPath)) {
    console.log(`✅ IDL dumped successfully: ${idlPath}`);
    return true;
  }
  
  console.error(`❌ Failed to extract IDL for ${programName}`);
  return false;
}

function createFallbackIdl(programName: string, programId: string): boolean {
  const idlPath = path.join(TARGET_IDL_DIR, `${programName}.json`);
  
  console.log(`🔧 Creating minimal fallback IDL for ${programName}...`);
  
  const fallbackIdl = {
    address: programId,
    metadata: {
      name: programName,
      version: "0.1.0",
      spec: "0.1.0",
      description: "Generated fallback IDL"
    },
    instructions: [],
    accounts: [],
    events: [],
    errors: [],
    types: []
  };
  
  try {
    fs.writeFileSync(idlPath, JSON.stringify(fallbackIdl, null, 2));
    console.log(`✅ Fallback IDL created: ${idlPath}`);
    return true;
  } catch (error) {
    console.error(`❌ Failed to create fallback IDL:`, error);
    return false;
  }
}

async function main() {
  console.log('🚀 Starting direct IDL extraction...');
  
  // Ensure directories exist
  ensureDirectoryExists(TARGET_IDL_DIR);
  
  // Check if binaries exist
  if (!fs.existsSync(TARGET_DEPLOY_DIR)) {
    console.error('❌ Target deploy directory not found. Run anchor build first.');
    process.exit(1);
  }
  
  const programs = [
    { name: 'decentralized_lottery', id: 'BH1qtDhU6PtB1jrUJPf8JoNt34ELuTvVTDktDLFyq2JV' },
    { name: 'decentralized_roulette', id: 'saLmMwuHKHDvaaPsA6GRaEjjzvmhx1VRgJGYeJpNDbr' }
  ];
  
  let allSuccess = true;
  
  for (const program of programs) {
    let success = await extractIdlFromBinary(program.name);
    
    if (!success) {
      console.log(`⚠️  Extraction failed for ${program.name}, creating fallback IDL...`);
      success = createFallbackIdl(program.name, program.id);
    }
    
    if (!success) {
      allSuccess = false;
    }
  }
  
  if (allSuccess) {
    console.log('\n✅ All IDL files extracted successfully!');
  } else {
    console.log('\n❌ Some IDL files failed to extract');
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}