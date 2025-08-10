import * as fs from 'fs';
import * as path from 'path';

const FRONTEND_DIR = '../crypto-lottery-frontend';
const FRONTEND_IDL_DIR = path.join(FRONTEND_DIR, 'src/lib/solana');
const TARGET_IDL_DIR = './target/idl';

function ensureDirectoryExists(dirPath: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`Created directory: ${dirPath}`);
  }
}

function copyIDL(programName: string) {
  const sourceFile = path.join(TARGET_IDL_DIR, `${programName}.json`);
  const targetFile = path.join(FRONTEND_IDL_DIR, `${programName}.json`);
  
  if (!fs.existsSync(sourceFile)) {
    console.error(`❌ IDL not found: ${sourceFile}`);
    return false;
  }
  
  try {
    fs.copyFileSync(sourceFile, targetFile);
    console.log(`✅ Synced ${programName}.json to frontend`);
    return true;
  } catch (error) {
    console.error(`❌ Failed to copy ${programName}.json:`, error);
    return false;
  }
}

function main() {
  console.log('🔄 Syncing IDL files to frontend...');
  
  ensureDirectoryExists(FRONTEND_IDL_DIR);
  
  const programs = ['decentralized_lottery', 'decentralized_roulette'];
  let allSuccess = true;
  
  for (const program of programs) {
    const success = copyIDL(program);
    if (!success) {
      allSuccess = false;
    }
  }
  
  if (allSuccess) {
    console.log('✅ All IDL files synced successfully!');
  } else {
    console.log('❌ Some IDL files failed to sync');
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}