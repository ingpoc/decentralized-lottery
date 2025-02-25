import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';

// Configuration
const PROJECT_ROOT = path.resolve(__dirname, '..');
const TARGET_DIR = path.join(PROJECT_ROOT, 'target');
const IDL_DIR = path.join(TARGET_DIR, 'idl');
const TYPES_DIR = path.join(TARGET_DIR, 'types');
const FRONTEND_DIR = path.join(PROJECT_ROOT, '..', 'stockanalysisgui');
const FRONTEND_LIB_DIR = path.join(FRONTEND_DIR, 'src', 'lib', 'solana');
const FRONTEND_TYPES_DIR = path.join(FRONTEND_DIR, 'src', 'types');

// File paths
const IDL_FILE = path.join(IDL_DIR, 'decentralized_lottery.json');
const TYPES_FILE = path.join(TYPES_DIR, 'decentralized_lottery.ts');
const FRONTEND_IDL_FILE = path.join(FRONTEND_LIB_DIR, 'decentralized_lottery.json');
const FRONTEND_TYPES_FILE = path.join(FRONTEND_TYPES_DIR, 'lottery_types.ts');

/**
 * Ensures a directory exists, creating it if necessary
 */
function ensureDirectoryExists(dirPath: string): void {
  if (!fs.existsSync(dirPath)) {
    fs.mkdirSync(dirPath, { recursive: true });
    console.log(`Created directory: ${dirPath}`);
  }
}

/**
 * Runs the anchor idl type command to generate TypeScript types
 */
function generateTypes(): void {
  try {
    console.log('Generating TypeScript types from IDL...');
    execSync(`anchor idl type -o ${TYPES_FILE} ${IDL_FILE}`, { 
      cwd: PROJECT_ROOT,
      stdio: 'inherit'
    });
    console.log('TypeScript types generated successfully.');
  } catch (error) {
    console.error('Error generating TypeScript types:', error);
    process.exit(1);
  }
}

/**
 * Copies the IDL file to the frontend directory
 */
function copyIdlToFrontend(): void {
  try {
    console.log(`Copying IDL file to frontend: ${FRONTEND_IDL_FILE}`);
    ensureDirectoryExists(path.dirname(FRONTEND_IDL_FILE));
    fs.copyFileSync(IDL_FILE, FRONTEND_IDL_FILE);
    console.log('IDL file copied successfully.');
  } catch (error) {
    console.error('Error copying IDL file:', error);
    process.exit(1);
  }
}

/**
 * Transforms the TypeScript types file to match the frontend format
 */
function transformAndCopyTypesToFrontend(): void {
  try {
    console.log(`Transforming and copying types to frontend: ${FRONTEND_TYPES_FILE}`);
    ensureDirectoryExists(path.dirname(FRONTEND_TYPES_FILE));
    
    // Read the generated types file
    let typesContent = fs.readFileSync(TYPES_FILE, 'utf8');
    
    // Transform the content for frontend use
    typesContent = typesContent
      // Add a comment header
      .replace(
        'export type DecentralizedLottery', 
        '/**\n * Program IDL in camelCase format in order to be used in JS/TS.\n *\n * Note that this is only a type helper and is not the actual IDL. The original\n * IDL can be found at `target/idl/decentralized_lottery.json`.\n */\nexport type DecentralizedLottery'
      );
    
    // Write the transformed content to the frontend types file
    fs.writeFileSync(FRONTEND_TYPES_FILE, typesContent);
    console.log('Types file transformed and copied successfully.');
  } catch (error) {
    console.error('Error transforming and copying types:', error);
    process.exit(1);
  }
}

/**
 * Main function to run the script
 */
function main(): void {
  console.log('Starting IDL and types update process...');
  
  // Check if IDL file exists
  if (!fs.existsSync(IDL_FILE)) {
    console.error(`IDL file not found: ${IDL_FILE}`);
    console.error('Please run "anchor build" first to generate the IDL.');
    process.exit(1);
  }
  
  // Generate TypeScript types
  generateTypes();
  
  // Copy files to frontend
  copyIdlToFrontend();
  transformAndCopyTypesToFrontend();
  
  console.log('IDL and types update completed successfully!');
}

// Run the script
main(); 