
import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';

// Configuration
const PROJECT_ROOT = path.resolve(__dirname, '..');
const TARGET_DIR = path.join(PROJECT_ROOT, 'target');
const IDL_DIR = path.join(TARGET_DIR, 'idl');
const TYPES_DIR = path.join(TARGET_DIR, 'types');
const FRONTEND_DIR = path.join(PROJECT_ROOT, '..', 'crypto-lottery-frontend');
const FRONTEND_LIB_DIR = path.join(FRONTEND_DIR, 'src', 'lib', 'solana');
const FRONTEND_TYPES_DIR = path.join(FRONTEND_DIR, 'src', 'types');

const PROGRAMS = [
  {
    name: 'decentralized_lottery',
    comprehensiveTypes: true,
  },
  {
    name: 'decentralized_roulette',
    comprehensiveTypes: false,
  },
];

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
function generateTypes(programName: string): void {
  const idlFile = path.join(IDL_DIR, `${programName}.json`);
  const typesFile = path.join(TYPES_DIR, `${programName}.ts`);
  try {
    console.log(`Generating TypeScript types for ${programName}...`);
    execSync(`anchor idl type -o ${typesFile} ${idlFile}`, {
      cwd: PROJECT_ROOT,
      stdio: 'inherit',
    });
    console.log(`TypeScript types for ${programName} generated successfully.`);
  } catch (error) {
    console.error(`Error generating TypeScript types for ${programName}:`, error);
    process.exit(1);
  }
}

/**
 * Copies the IDL file to the frontend directory
 */
function copyIdlToFrontend(programName: string): void {
  const idlFile = path.join(IDL_DIR, `${programName}.json`);
  const frontendIdlFile = path.join(FRONTEND_LIB_DIR, `${programName}.json`);
  try {
    console.log(`Copying IDL file to frontend: ${frontendIdlFile}`);
    ensureDirectoryExists(path.dirname(frontendIdlFile));
    fs.copyFileSync(idlFile, frontendIdlFile);
    console.log('IDL file copied successfully.');
  } catch (error) {
    console.error('Error copying IDL file:', error);
    process.exit(1);
  }
}

/**
 * Copies the auto-generated types to frontend for reference
 */
function copyAutoTypesToFrontend(programName: string): void {
  const typesFile = path.join(TYPES_DIR, `${programName}.ts`);
  const frontendAutoTypesFile = path.join(FRONTEND_TYPES_DIR, `${programName}.ts`);
  try {
    console.log(`Copying auto-generated types to frontend: ${frontendAutoTypesFile}`);
    ensureDirectoryExists(path.dirname(frontendAutoTypesFile));

    // Read the generated types file
    let typesContent = fs.readFileSync(typesFile, 'utf8');

    // Write the transformed content to the frontend auto-types file
    fs.writeFileSync(frontendAutoTypesFile, typesContent);
    console.log('Auto-generated types copied successfully.');
  } catch (error) {
    console.error('Error copying auto-generated types:', error);
    process.exit(1);
  }
}

/**
 * Main function to run the script
 */
function main(): void {
  console.log('Starting IDL and types update process...');

  for (const program of PROGRAMS) {
    const idlFile = path.join(IDL_DIR, `${program.name}.json`);

    // Check if IDL file exists
    if (!fs.existsSync(idlFile)) {
      console.error(`IDL file not found: ${idlFile}`);
      console.error('Please run "anchor build" first to generate the IDL.');
      process.exit(1);
    }

    // Generate TypeScript types
    generateTypes(program.name);

    // Copy files to frontend
    copyIdlToFrontend(program.name);
    copyAutoTypesToFrontend(program.name);
  }

  console.log('IDL and types update completed successfully!');
}

// Run the script
main();
