#!/usr/bin/env ts-node

/**
 * IDL Compatibility Validation Script
 * Automated dependency and configuration compatibility checker
 * Run: npm run validate-idl-compatibility
 */

import * as fs from 'fs';
import * as path from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

interface ValidationResult {
  category: string;
  status: 'pass' | 'warn' | 'fail';
  message: string;
  details?: string;
  fix?: string;
}

class IDLCompatibilityValidator {
  private results: ValidationResult[] = [];
  private projectRoot: string;

  constructor() {
    this.projectRoot = process.cwd();
  }

  private addResult(result: ValidationResult) {
    this.results.push(result);
    
    const icon = result.status === 'pass' ? '✅' : result.status === 'warn' ? '⚠️' : '❌';
    console.log(`${icon} [${result.category}] ${result.message}`);
    
    if (result.details) {
      console.log(`   ${result.details}`);
    }
    
    if (result.fix && result.status !== 'pass') {
      console.log(`   💡 Fix: ${result.fix}`);
    }
  }

  async validateAnchorVersions(): Promise<void> {
    try {
      const anchorTomlPath = path.join(this.projectRoot, 'Anchor.toml');
      const cargoTomlPath = path.join(this.projectRoot, 'Cargo.toml');
      const packageJsonPath = path.join(this.projectRoot, 'package.json');

      if (!fs.existsSync(anchorTomlPath)) {
        this.addResult({
          category: 'Anchor Config',
          status: 'fail',
          message: 'Anchor.toml not found',
          fix: 'Initialize Anchor project with `anchor init`'
        });
        return;
      }

      // Read Anchor.toml version
      const anchorToml = fs.readFileSync(anchorTomlPath, 'utf-8');
      const anchorVersionMatch = anchorToml.match(/anchor_version\s*=\s*"([^"]+)"/);
      const anchorVersion = anchorVersionMatch?.[1];

      // Read Cargo.toml anchor-lang version
      const cargoToml = fs.readFileSync(cargoTomlPath, 'utf-8');
      const cargoAnchorMatch = cargoToml.match(/anchor-lang.*version\s*=\s*"([^"]+)"/);
      const cargoAnchorVersion = cargoAnchorMatch?.[1]?.replace(/[~^]/, '');

      // Read package.json @coral-xyz/anchor version
      const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf-8'));
      const packageAnchorVersion = packageJson.dependencies?.['@coral-xyz/anchor']?.replace(/[~^]/, '');

      if (anchorVersion && cargoAnchorVersion && anchorVersion === cargoAnchorVersion) {
        this.addResult({
          category: 'Version Consistency',
          status: 'pass',
          message: `Anchor versions consistent (${anchorVersion})`,
        });
      } else {
        this.addResult({
          category: 'Version Consistency',
          status: 'fail',
          message: 'Anchor version mismatch detected',
          details: `Anchor.toml: ${anchorVersion}, Cargo.toml: ${cargoAnchorVersion}`,
          fix: 'Update versions to match in both files'
        });
      }

      if (packageAnchorVersion && packageAnchorVersion !== anchorVersion) {
        this.addResult({
          category: 'Package.json',
          status: 'warn',
          message: 'Package.json Anchor version differs from project version',
          details: `package.json: ${packageAnchorVersion}, project: ${anchorVersion}`,
          fix: 'Update package.json to match project version'
        });
      }
    } catch (error) {
      this.addResult({
        category: 'Anchor Config',
        status: 'fail',
        message: 'Error reading configuration files',
        details: error instanceof Error ? error.message : String(error)
      });
    }
  }

  async validateProblematicDependencies(): Promise<void> {
    try {
      const cargoTomlPath = path.join(this.projectRoot, 'Cargo.toml');
      const cargoToml = fs.readFileSync(cargoTomlPath, 'utf-8');

      // Check proc_macro2 version
      const procMacro2Match = cargoToml.match(/proc-macro2\s*=\s*"([^"]+)"/);
      if (procMacro2Match) {
        const version = procMacro2Match[1].replace(/[=~^]/, '');
        const majorMinor = version.split('.').slice(0, 2).join('.');
        const patch = parseInt(version.split('.')[2] || '0');

        if (majorMinor === '1.0' && patch >= 95) {
          this.addResult({
            category: 'Dependencies',
            status: 'fail',
            message: `Problematic proc_macro2 version detected (${version})`,
            details: 'Versions 1.0.95+ cause IDL generation failures',
            fix: 'Use proc-macro2 = "=1.0.94" in Cargo.toml'
          });
        } else {
          this.addResult({
            category: 'Dependencies',
            status: 'pass',
            message: `proc_macro2 version safe (${version})`
          });
        }
      }

      // Check for explicit solana-program dependency
      if (cargoToml.includes('solana-program')) {
        this.addResult({
          category: 'Dependencies',
          status: 'warn',
          message: 'Explicit solana-program dependency detected',
          details: 'This can conflict with anchor-lang\'s solana-program',
          fix: 'Remove solana-program and use anchor_lang::solana_program instead'
        });
      }

      // Check for anchor-spl without workspace reference
      const anchorSplMatch = cargoToml.match(/anchor-spl\s*=\s*"([^"]+)"/);
      if (anchorSplMatch && !cargoToml.includes('anchor-spl = { workspace = true }')) {
        this.addResult({
          category: 'Dependencies',
          status: 'warn',
          message: 'anchor-spl version should use workspace reference',
          fix: 'Use anchor-spl = { workspace = true } in program Cargo.toml files'
        });
      }

    } catch (error) {
      this.addResult({
        category: 'Dependencies',
        status: 'fail',
        message: 'Error validating dependencies',
        details: error instanceof Error ? error.message : String(error)
      });
    }
  }

  async validateProgramFeatures(): Promise<void> {
    try {
      const programsDir = path.join(this.projectRoot, 'programs');
      if (!fs.existsSync(programsDir)) {
        return;
      }

      const programs = fs.readdirSync(programsDir, { withFileTypes: true })
        .filter(dirent => dirent.isDirectory())
        .map(dirent => dirent.name);

      for (const program of programs) {
        const programCargoPath = path.join(programsDir, program, 'Cargo.toml');
        if (!fs.existsSync(programCargoPath)) {
          continue;
        }

        const programCargo = fs.readFileSync(programCargoPath, 'utf-8');
        
        // Check if program uses anchor-spl
        if (programCargo.includes('anchor-spl')) {
          // Check if idl-build feature is properly configured
          const featuresSection = programCargo.match(/\[features\]([\s\S]*?)(?=\[|$)/)?.[1] || '';
          
          if (!featuresSection.includes('idl-build') || !featuresSection.includes('anchor-spl/idl-build')) {
            this.addResult({
              category: 'Program Features',
              status: 'warn',
              message: `${program} missing idl-build feature`,
              details: 'anchor-spl programs should include idl-build feature',
              fix: 'Add idl-build = ["anchor-spl/idl-build"] to [features] section'
            });
          } else {
            this.addResult({
              category: 'Program Features',
              status: 'pass',
              message: `${program} idl-build feature configured correctly`
            });
          }
        }
      }
    } catch (error) {
      this.addResult({
        category: 'Program Features',
        status: 'fail',
        message: 'Error validating program features',
        details: error instanceof Error ? error.message : String(error)
      });
    }
  }

  async validateRustCode(): Promise<void> {
    try {
      const programsDir = path.join(this.projectRoot, 'programs');
      if (!fs.existsSync(programsDir)) {
        return;
      }

      // Find all .rs files in programs directory
      const findRustFiles = async (dir: string): Promise<string[]> => {
        const files: string[] = [];
        const entries = fs.readdirSync(dir, { withFileTypes: true });
        
        for (const entry of entries) {
          const fullPath = path.join(dir, entry.name);
          if (entry.isDirectory()) {
            files.push(...await findRustFiles(fullPath));
          } else if (entry.name.endsWith('.rs')) {
            files.push(fullPath);
          }
        }
        
        return files;
      };

      const rustFiles = await findRustFiles(programsDir);
      let issuesFound = 0;

      for (const file of rustFiles) {
        const content = fs.readFileSync(file, 'utf-8');
        const relativePath = path.relative(this.projectRoot, file);

        // Check for problematic patterns
        if (content.includes('Option<Account<\'info, Mint>>')) {
          this.addResult({
            category: 'Code Patterns',
            status: 'warn',
            message: `Potentially problematic pattern in ${relativePath}`,
            details: 'Option<Account<\'info, Mint>> can cause IDL generation issues',
            fix: 'Consider using Pubkey or making the account non-optional'
          });
          issuesFound++;
        }

        if (content.includes('Option<Account<\'info, TokenAccount>>')) {
          this.addResult({
            category: 'Code Patterns',
            status: 'warn',
            message: `Potentially problematic pattern in ${relativePath}`,
            details: 'Option<Account<\'info, TokenAccount>> can cause IDL generation issues',
            fix: 'Consider using Pubkey or making the account non-optional'
          });
          issuesFound++;
        }

        // Check for large structs (potential stack overflow)
        const structMatches = content.match(/#\[derive\(Accounts\)\][^}]*?struct\s+\w+[^}]*?\{([^}]*)\}/gs);
        if (structMatches) {
          for (const match of structMatches) {
            const fieldCount = (match.match(/pub\s+\w+:/g) || []).length;
            if (fieldCount > 20) {
              this.addResult({
                category: 'Code Patterns',
                status: 'warn',
                message: `Large struct detected in ${relativePath}`,
                details: `${fieldCount} fields - may cause stack overflow`,
                fix: 'Consider using Box<> for large account data structures'
              });
              issuesFound++;
            }
          }
        }
      }

      if (issuesFound === 0) {
        this.addResult({
          category: 'Code Patterns',
          status: 'pass',
          message: 'No problematic code patterns detected'
        });
      }

    } catch (error) {
      this.addResult({
        category: 'Code Patterns',
        status: 'fail',
        message: 'Error validating Rust code',
        details: error instanceof Error ? error.message : String(error)
      });
    }
  }

  async validateCompilation(): Promise<void> {
    try {
      console.log('\n🧪 Running compilation test...');
      
      const { stdout, stderr } = await execAsync('cargo check --workspace --all-features', {
        cwd: this.projectRoot,
        timeout: 60000 // 1 minute timeout
      });

      this.addResult({
        category: 'Compilation',
        status: 'pass',
        message: 'Compilation test passed successfully'
      });

    } catch (error: any) {
      this.addResult({
        category: 'Compilation',
        status: 'fail',
        message: 'Compilation test failed',
        details: 'Run "cargo check --workspace" for detailed error information',
        fix: 'Fix compilation errors before proceeding with IDL generation'
      });
    }
  }

  async generateReport(): Promise<void> {
    console.log('\n' + '='.repeat(60));
    console.log('          IDL COMPATIBILITY VALIDATION REPORT');
    console.log('='.repeat(60));

    const passed = this.results.filter(r => r.status === 'pass').length;
    const warned = this.results.filter(r => r.status === 'warn').length;
    const failed = this.results.filter(r => r.status === 'fail').length;

    console.log(`\n📊 Summary: ${passed} passed, ${warned} warnings, ${failed} failed\n`);

    if (failed > 0) {
      console.log('❌ CRITICAL ISSUES FOUND:');
      this.results.filter(r => r.status === 'fail').forEach(result => {
        console.log(`   • ${result.message}`);
        if (result.fix) {
          console.log(`     💡 ${result.fix}`);
        }
      });
      console.log('\n🚨 These issues MUST be fixed to prevent IDL generation failures!\n');
    }

    if (warned > 0) {
      console.log('⚠️  WARNINGS:');
      this.results.filter(r => r.status === 'warn').forEach(result => {
        console.log(`   • ${result.message}`);
        if (result.fix) {
          console.log(`     💡 ${result.fix}`);
        }
      });
      console.log('\n💡 Consider addressing these warnings to improve reliability.\n');
    }

    if (passed > 0 && failed === 0) {
      console.log('✅ All critical validations passed! IDL generation should work correctly.\n');
    }

    // Exit with appropriate code
    process.exit(failed > 0 ? 1 : 0);
  }

  async run(): Promise<void> {
    console.log('🚀 Starting IDL Compatibility Validation...\n');

    await this.validateAnchorVersions();
    await this.validateProblematicDependencies();
    await this.validateProgramFeatures();
    await this.validateRustCode();
    await this.validateCompilation();

    await this.generateReport();
  }
}

// Run the validator
if (require.main === module) {
  const validator = new IDLCompatibilityValidator();
  validator.run().catch((error) => {
    console.error('❌ Validation script failed:', error);
    process.exit(1);
  });
}

export default IDLCompatibilityValidator;