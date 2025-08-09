# Claude Code Hooks Configuration

This directory contains Claude Code hooks that prevent common development issues in Solana/Anchor projects.

## Installed Hooks

### 1. Version Guard (`version-guard.sh`)
**Purpose**: Prevents version mismatches that cause build failures

**Triggers**:
- `pre-edit`: When editing `Anchor.toml` or `package.json`
- `pre-bash`: Before running anchor/build/deploy commands  
- `session-start`: When starting a new development session

**What it checks**:
- ✅ Anchor CLI version matches `Anchor.toml` `anchor_version`
- ✅ Package.json dependency versions are compatible
- ✅ Program ID configuration consistency
- ✅ Node.js version requirements

**Example prevention**:
```bash
🚨 Attempting to set anchor_version to '0.28.0'
🚨 But installed CLI is '0.30.1' 
🚨 This will cause build failures!
```

### 2. Project Validator (`project-validator.sh`)
**Purpose**: Validates overall project health and catches development issues

**Triggers**:
- `pre-bash`: Before deployment/build commands
- `session-start`: Project startup validation

**What it checks**:
- ✅ Solana CLI installation and wallet setup
- ✅ `.gitignore` configuration for Solana projects  
- ✅ Network configuration (localnet/devnet/mainnet)
- ✅ Test validator status for localnet
- ✅ Large struct detection (stack overflow prevention)
- ✅ Backup file cleanup suggestions

## Configuration

Hooks are configured in `.claude/settings.json`:

```json
{
  "hooks": {
    "pre-edit": ".claude/hooks/version-guard.sh",
    "pre-bash": ".claude/hooks/version-guard.sh", 
    "session-start": ".claude/hooks/version-guard.sh"
  }
}
```

## Testing Hooks

You can test hooks manually:

```bash
# Test version validation
CLAUDE_HOOK_TYPE="session-start" ./.claude/hooks/version-guard.sh

# Test project validation  
CLAUDE_HOOK_TYPE="session-start" ./.claude/hooks/project-validator.sh

# Simulate version mismatch
CLAUDE_HOOK_TYPE="pre-edit" CLAUDE_FILE_PATH="Anchor.toml" \
  CLAUDE_EDIT_CONTENT='anchor_version = "0.28.0"' \
  ./.claude/hooks/version-guard.sh
```

## Issues Prevented

Based on actual development experience, these hooks prevent:

1. **Stack Overflow Build Errors** - Detected through large struct analysis
2. **Version Mismatches** - CLI vs config vs dependencies  
3. **Network Configuration Issues** - Wrong cluster settings
4. **Program ID Inconsistencies** - Centralized management validation
5. **Environment Setup Problems** - Missing tools or wallets
6. **File System Clutter** - Backup file detection

## Hook Environment Variables

Hooks receive these environment variables from Claude Code:

- `CLAUDE_HOOK_TYPE`: Type of hook (pre-edit, pre-bash, session-start)
- `CLAUDE_FILE_PATH`: Path to file being edited (for pre-edit)
- `CLAUDE_EDIT_CONTENT`: New file content (for pre-edit)
- `CLAUDE_COMMAND`: Command being executed (for pre-bash)

## Customization

To add custom validations:

1. **Extend existing hooks**: Add functions to `version-guard.sh` or `project-validator.sh`
2. **Create new hooks**: Add new `.sh` files and update `.claude/settings.json`
3. **Project-specific checks**: Add validations specific to your program logic

## Troubleshooting

**Hook not running**:
- Check file permissions: `chmod +x .claude/hooks/*.sh`
- Verify settings.json syntax
- Test hooks manually as shown above

**Hook blocking valid changes**:
- Review hook logic in the respective `.sh` file
- Temporarily disable by commenting out in `settings.json`
- Report false positives for improvement

## Integration with Centralized Program ID Management

The hooks integrate with the existing `scripts/update-program-ids.ts` system:

- Detects when program source files are modified
- Suggests running `npm run update-ids all <network>`
- Validates program ID consistency across files
- Prevents deployment with mismatched IDs

This creates a complete workflow protection system for Solana development.