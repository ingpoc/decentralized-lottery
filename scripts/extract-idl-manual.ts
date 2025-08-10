import { exec } from 'child_process';
import * as fs from 'fs';
import * as path from 'path';
import { promisify } from 'util';

const execAsync = promisify(exec);

async function extractIDLFromProgram(programName: string): Promise<boolean> {
    const idlPath = `./target/idl/${programName}.json`;
    const programPath = `./target/sbf-solana-solana/release/${programName}.so`;
    
    console.log(`Extracting IDL for ${programName}...`);
    console.log(`Program path: ${programPath}`);
    console.log(`IDL path: ${idlPath}`);
    
    // Check if the program binary exists
    if (!fs.existsSync(programPath)) {
        console.error(`Program binary not found: ${programPath}`);
        return false;
    }
    
    try {
        // Use anchor to extract IDL from the binary
        const { stdout, stderr } = await execAsync(
            `anchor idl parse ${programPath} -o ${idlPath}`,
            { 
                env: {
                    ...process.env,
                    PATH: `${process.env.HOME}/.cargo/bin:${process.env.HOME}/.local/share/solana/install/active_release/bin:${process.env.PATH}`
                }
            }
        );
        
        if (stderr) {
            console.log(`stderr: ${stderr}`);
        }
        
        if (fs.existsSync(idlPath)) {
            console.log(`✅ IDL extracted successfully: ${idlPath}`);
            return true;
        } else {
            console.error(`❌ IDL file not created: ${idlPath}`);
            return false;
        }
        
    } catch (error) {
        console.error(`Error extracting IDL for ${programName}:`, error);
        return false;
    }
}

async function main() {
    console.log('🔧 Manual IDL extraction starting...');
    
    // Ensure target directories exist
    if (!fs.existsSync('./target/idl')) {
        fs.mkdirSync('./target/idl', { recursive: true });
    }
    
    const programs = ['decentralized_lottery', 'decentralized_roulette'];
    let success = true;
    
    for (const program of programs) {
        const result = await extractIDLFromProgram(program);
        if (!result) {
            success = false;
        }
    }
    
    if (success) {
        console.log('✅ All IDLs extracted successfully!');
    } else {
        console.log('❌ Some IDL extractions failed');
        process.exit(1);
    }
}

if (require.main === module) {
    main().catch((error) => {
        console.error('Fatal error:', error);
        process.exit(1);
    });
}