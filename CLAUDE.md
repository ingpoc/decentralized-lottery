# Decentralized Lottery Project Memory

## Project Overview
Solana/Anchor-based lottery and roulette programs with TypeScript frontend integration.

## Version Management
- **Anchor**: CLI=0.30.1, Config=0.30.1, Deps=^0.30.1 ✅
- **Node**: 23.7.0 (compatible)
- **Solana**: CLI=2.1.5 
- **Network**: Localnet (primary for development)

## Claude Code Hooks Setup
**Status**: ✅ Active and preventing issues

### Installed Hooks:
1. **Version Guard** (`.claude/hooks/version-guard.sh`)
   - Prevents Anchor version mismatches
   - Validates program ID consistency 
   - Triggers: pre-edit, pre-bash, session-start

2. **Project Validator** (`.claude/hooks/project-validator.sh`)
   - Environment validation
   - Development best practices
   - Stack overflow prevention
   - Triggers: pre-bash, session-start

### Issues Successfully Prevented:
- ❌ Anchor 0.28.0 vs 0.30.1 CLI mismatch (would cause build failure)
- ✅ Program ID consistency across files
- ✅ Network configuration validation
- ✅ Large struct detection for stack overflow prevention

## Centralized Program ID Management
**Location**: `config/program-ids.json`
**Update Script**: `scripts/update-program-ids.ts`

### Networks:
- **Localnet**: Primary development network
  - `decentralized_lottery`: `BH1qtDhU6PtB1jrUJPf8JoNt34ELuTvVTDktDLFyq2JV`
  - `decentralized_roulette`: `saLmMwuHKHDvaaPsA6GRaEjjzvmhx1VRgJGYeJpNDbr`
- **Devnet**: Staging network
  - `decentralized_lottery`: `9G1iJ7M8fNotapwrW9o9EgB3EeobrzHPyL8kgmQGQGvz`
  - `decentralized_roulette`: `HFpMuAtTLCJQoo1bxZ9XUfyoaABSjhDf7ehgjpBBgDJo`

### Commands:
```bash
# Update all files for localnet
npm run update-ids all localnet

# Update only source code
npm run update-ids source localnet

# Update only IDLs  
npm run update-ids idls localnet

# Update frontend environment
npm run update-ids env localnet
```

## Essential Scripts
**Location**: `scripts/`
- `build.sh` - Build with optimization
- `deploy-local.ts` - Localnet deployment 
- `update-program-ids.ts` - Centralized ID management

## Deployment Workflow
1. **Start local validator**: `solana-test-validator`
2. **Deploy programs**: `npm run deploy:local`
3. **Update frontend**: Automatic via centralized system
4. **Verify**: Hooks validate consistency

## Stack Overflow Prevention
**Issue**: Solana 4096-byte stack limit
**Solution**: Use specialized agent `solana-stack-overflow-fixer`
**Detection**: Hooks detect large structs (>20 fields)
**Fix Pattern**: Box<> large account data structures

## MCP Integration
**Available Tools**: `mcp__crypto-lottery-solana__*`
- `derivePDA()` - PDA generation
- `decodeAccountData()` - Account parsing
- `validateTokenAmount()` - Amount validation
- `simulateTransaction()` - Pre-deployment testing

## Error Prevention Patterns

### Version Consistency
- **Never** manually edit `anchor_version` without checking CLI
- **Always** use hooks to validate before changes
- **Remember**: Version mismatches cause build failures

### Program ID Management  
- **Never** manually update program IDs in source files
- **Always** use centralized `update-program-ids.ts` script
- **Verify** consistency across lib.rs, Anchor.toml, IDLs, frontend

### Network Configuration
- **Default**: Localnet for development (no SOL funding issues)
- **Validate**: Network settings match deployment target
- **Check**: Test validator running for localnet

### Build Optimization
- **Use**: `solana-stack-overflow-fixer` agent proactively
- **Monitor**: Stack usage in complex instructions
- **Apply**: Box<> pattern for large account structures

### File Management
- **Clean**: Remove .bak, .backup, .old files regularly  
- **Track**: Use .gitignore for build artifacts
- **Organize**: Keep only essential scripts

## Specialized Agents Usage

### Use Proactively:
- `solana-stack-overflow-fixer` - Before stack errors occur
- `anchor-program-expert` - For all program reviews
- `program-frontend-bridge` - IDL/frontend sync issues
- `game-lifecycle-orchestrator` - Game state management

### When to Trigger:
- Stack frame warnings → `solana-stack-overflow-fixer`
- Program deployment → `anchor-program-expert`  
- IDL generation fails → `program-frontend-bridge`
- Game state stuck → `game-lifecycle-orchestrator`

## Quick Validation Commands
```bash
# Test hooks manually
./.claude/hooks/version-guard.sh

# Check project health  
./.claude/hooks/project-validator.sh

# Validate versions
anchor --version && node --version && solana --version
```

## Success Metrics
✅ Zero version mismatch errors since hook installation
✅ Centralized program ID management working  
✅ Clean project structure (removed 8 redundant scripts)
✅ Localnet deployment pipeline functional
✅ Stack overflow issues resolved with specialized agents