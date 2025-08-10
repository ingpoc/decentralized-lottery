import { exec, spawn } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import { promisify } from 'util';

const execAsync = promisify(exec);

const TARGET_IDL_DIR = './target/idl';
const TARGET_TYPES_DIR = './target/types';

function ensureDirectoryExists(dirPath: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`Created directory: ${dirPath}`);
  }
}

async function runCommand(command: string, args: string[], description: string): Promise<boolean> {
  return new Promise((resolve) => {
    console.log(`🔄 ${description}...`);
    console.log(`Running: ${command} ${args.join(' ')}`);
    
    const child = spawn(command, args, {
      stdio: 'inherit',
      env: {
        ...process.env,
        PATH: `${process.env.HOME}/.cargo/bin:${process.env.HOME}/.local/share/solana/install/active_release/bin:${process.env.PATH}`
      }
    });
    
    child.on('close', (code) => {
      if (code === 0) {
        console.log(`✅ ${description} completed successfully`);
        resolve(true);
      } else {
        console.error(`❌ ${description} failed with code ${code}`);
        resolve(false);
      }
    });
    
    child.on('error', (error) => {
      console.error(`❌ ${description} error:`, error.message);
      resolve(false);
    });
  });
}

async function generateIDLForProgram(programName: string): Promise<boolean> {
  const idlFile = path.join(TARGET_IDL_DIR, `${programName}.json`);
  
  // Strategy 1: Try anchor idl build
  console.log(`\n🔧 Generating IDL for ${programName}...`);
  
  let success = await runCommand('anchor', ['idl', 'build', '-p', programName], `IDL build for ${programName}`);
  
  if (success && fs.existsSync(idlFile)) {
    console.log(`✅ IDL generated successfully: ${idlFile}`);
    return true;
  }
  
  // Strategy 2: Try anchor build then idl extract
  console.log(`\n🔄 Fallback: Building program and extracting IDL...`);
  success = await runCommand('anchor', ['build', '-p', programName], `Program build for ${programName}`);
  
  if (success) {
    success = await runCommand('anchor', ['idl', 'build', '-p', programName], `IDL extract for ${programName}`);
    
    if (success && fs.existsSync(idlFile)) {
      console.log(`✅ IDL extracted successfully: ${idlFile}`);
      return true;
    }
  }
  
  // Strategy 3: Try to extract from deployed program (if program ID exists)
  try {
    const anchorToml = fs.readFileSync('./Anchor.toml', 'utf-8');
    const programIdMatch = anchorToml.match(new RegExp(`${programName}\\s*=\\s*"([^"]+)"`));
    
    if (programIdMatch) {
      const programId = programIdMatch[1];
      console.log(`\n🔄 Fallback: Fetching IDL from deployed program ${programId}...`);
      
      success = await runCommand('anchor', [
        'idl', 'fetch', programId, 
        '-o', idlFile,
        '--provider.cluster', 'localnet'
      ], `IDL fetch for ${programName}`);
      
      if (success && fs.existsSync(idlFile)) {
        console.log(`✅ IDL fetched successfully: ${idlFile}`);
        return true;
      }
    }
  } catch (error) {
    console.log(`⚠️  Could not read Anchor.toml or extract program ID: ${error}`);
  }
  
  console.error(`❌ All strategies failed for ${programName}`);
  return false;
}

async function main() {
  console.log('🚀 Starting IDL generation fallback process...');
  
  // Ensure directories exist
  ensureDirectoryExists(TARGET_IDL_DIR);
  ensureDirectoryExists(TARGET_TYPES_DIR);
  
  const programs = ['decentralized_lottery', 'decentralized_roulette'];
  let allSuccess = true;
  
  for (const program of programs) {
    const success = await generateIDLForProgram(program);
    if (!success) {
      allSuccess = false;
    }
  }
  
  if (allSuccess) {
    console.log('\n✅ All IDL files generated successfully!');
    console.log('🔄 Running sync to frontend...');
    
    // Try to run sync-idl script
    try {
      await execAsync('npm run sync-idl');
      console.log('✅ IDL files synced to frontend!');
    } catch (error) {
      console.error('❌ Failed to sync IDL files:', error);
    }
  } else {
    console.log('\n❌ Some IDL files failed to generate');
    process.exit(1);
  }
}

if (require.main === module) {
  main().catch((error) => {
    console.error('Fatal error:', error);
    process.exit(1);
  });
}