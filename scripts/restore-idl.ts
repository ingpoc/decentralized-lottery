import * as fs from 'fs';
import * as path from 'path';

const TARGET_IDL_DIR = './target/idl';
const FRONTEND_IDL_DIR = '../crypto-lottery-frontend/src/lib/solana';

function ensureDirectoryExists(dirPath: string) {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`Created directory: ${dirPath}`);
  }
}

function copyWorkingIdls() {
  console.log('🔄 Restoring working IDL files...');
  
  ensureDirectoryExists(TARGET_IDL_DIR);
  
  const programs = ['decentralized_lottery', 'decentralized_roulette'];
  let allSuccess = true;
  
  for (const program of programs) {
    const sourceFile = path.join(FRONTEND_IDL_DIR, `${program}.json`);
    const targetFile = path.join(TARGET_IDL_DIR, `${program}.json`);
    
    if (fs.existsSync(sourceFile)) {
      try {
        fs.copyFileSync(sourceFile, targetFile);
        console.log(`✅ Restored ${program}.json to target/idl/`);
      } catch (error) {
        console.error(`❌ Failed to restore ${program}.json:`, error);
        allSuccess = false;
      }
    } else {
      console.error(`❌ Source IDL not found: ${sourceFile}`);
      allSuccess = false;
    }
  }
  
  return allSuccess;
}

function main() {
  const success = copyWorkingIdls();
  
  if (success) {
    console.log('\n✅ All working IDL files restored successfully!');
  } else {
    console.log('\n❌ Some IDL files failed to restore');
    process.exit(1);
  }
}

if (require.main === module) {
  main();
}