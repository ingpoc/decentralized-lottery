#!/bin/bash

# MCP Schema Validation Hook
# Validates changes against MCP server schemas and program interfaces

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [MCP-SCHEMA]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [MCP-SCHEMA]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [MCP-SCHEMA]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [MCP-SCHEMA]${NC} $1"
}

# Function to validate IDL schema consistency
validate_idl_schema() {
    local file_path="$1"
    local content="$2"
    
    # Check if this affects IDL-generating files
    if [[ "$file_path" =~ programs/.*/src/(lib|state|instructions)/.*\.rs$ ]]; then
        log_info "Validating changes against IDL schema requirements..."
        
        # Check for account structure changes that require IDL updates
        if echo "$content" | grep -q "#\[account\]" || echo "$content" | grep -q "pub struct.*Accounts"; then
            log_warning "Account structure change detected"
            log_info "🔄 Consider running: anchor build && npm run update-ids all localnet"
            log_info "IDL regeneration may be required for frontend synchronization"
        fi
        
        # Check for instruction changes that affect IDL
        if echo "$content" | grep -q "pub fn handler" && [[ "$file_path" =~ instructions/.*\.rs$ ]]; then
            log_info "Instruction handler change detected - IDL may need updates"
        fi
        
        # Validate account field types are IDL-compatible
        if echo "$content" | grep -q "pub.*: HashMap\|pub.*: BTreeMap"; then
            log_error "BLOCKED: Complex types detected that may not serialize properly in IDL"
            log_error "HashMap and BTreeMap are not directly supported in Anchor IDL"
            echo ""
            log_info "✅ Solutions:"
            log_info "1. Use Vec<(Key, Value)> instead of HashMap"
            log_info "2. Create custom serializable structs"
            log_info "3. Use anchor_lang supported types only"
            echo ""
            exit 1
        fi
        
        log_success "IDL schema validation passed"
    fi
}

# Function to validate MCP tool compatibility
validate_mcp_compatibility() {
    local file_path="$1"
    local content="$2"
    
    # Check for changes that might affect MCP tool integration
    if [[ "$file_path" =~ Anchor\.toml$|package\.json$ ]]; then
        log_info "Configuration file change - validating MCP tool compatibility..."
        
        # Check for program ID changes that affect MCP tools
        if [[ "$file_path" =~ Anchor\.toml$ ]] && echo "$content" | grep -q "\[programs\.\(localnet\|devnet\|mainnet\)\]"; then
            log_warning "Program ID configuration change detected"
            log_info "MCP tools may need program ID updates for blockchain operations"
        fi
        
        # Check for dependency changes that might affect TypeScript generation
        if [[ "$file_path" =~ package\.json$ ]] && echo "$content" | grep -q "\"@coral-xyz/anchor\"\|\"@solana/web3.js\""; then
            log_warning "Anchor/Solana dependency change detected"
            log_info "May require TypeScript type regeneration"
        fi
    fi
    
    # Validate account naming conventions for MCP tools
    if echo "$content" | grep -q "#\[account\]" && echo "$content" | grep -q "pub struct [a-z]"; then
        log_error "BLOCKED: Account struct with lowercase name detected"
        log_error "Account structs should use PascalCase for MCP tool compatibility"
        echo ""
        log_info "✅ Solution: Use PascalCase for account structs:"
        log_info "   Before: pub struct lottery_account {"
        log_info "   After:  pub struct LotteryAccount {"
        echo ""
        exit 1
    fi
}

# Function to validate token operations
validate_token_operations() {
    local file_path="$1"
    local content="$2"
    
    # Check for token operations that need proper validation
    if echo "$content" | grep -q "spl_token::\|token::" && [[ "$file_path" =~ instructions/.*\.rs$ ]]; then
        log_info "Token operation detected - validating patterns..."
        
        # Check for hardcoded token amounts
        if echo "$content" | grep -q "[0-9]\{6,\}" && echo "$content" | grep -q "amount\|transfer"; then
            log_warning "Large numeric values detected in token operations"
            log_info "💡 Consider using MCP crypto-lottery tools for proper amount conversion:"
            log_info "   - toTokenAmount() for UI to token amount conversion"
            log_info "   - validateTokenAmount() for operation validation"
        fi
        
        # Check for missing decimal handling
        if echo "$content" | grep -q "transfer" && ! echo "$content" | grep -q "decimals\|DECIMALS"; then
            log_warning "Token transfer without explicit decimal handling"
            log_info "Ensure proper decimal conversion for token operations"
        fi
    fi
}

# Function to validate PDA derivation patterns
validate_pda_operations() {
    local file_path="$1"
    local content="$2"
    
    # Check for PDA operations
    if echo "$content" | grep -q "seeds = \[" || echo "$content" | grep -q "Pubkey::find_program_address"; then
        log_info "PDA operation detected - validating derivation patterns..."
        
        # Check for consistent seed ordering
        if echo "$content" | grep -q "seeds = \[.*authority.*,.*nonce" && echo "$content" | grep -q "seeds = \[.*nonce.*,.*authority"; then
            log_error "BLOCKED: Inconsistent PDA seed ordering detected"
            log_error "Mixed seed orders will cause 'PDA not found' errors"
            echo ""
            log_info "✅ Solution: Use consistent seed ordering across all PDAs:"
            log_info "   Standard order: [SEED_CONSTANT, authority.key(), nonce.to_le_bytes()]"
            echo ""
            exit 1
        fi
        
        # Check for missing seed constants
        if echo "$content" | grep -q 'seeds = \[b"[^"]*"' && ! echo "$content" | grep -q "SEED\|_SEED"; then
            log_warning "Hardcoded seed strings detected"
            log_info "💡 Consider using seed constants for better maintainability"
            log_info "💡 Use MCP derivePDA() tool for consistent PDA derivation"
        fi
    fi
}

# Function to validate network configuration consistency
validate_network_config() {
    local file_path="$1"
    local content="$2"
    
    # Check for network-specific configurations
    if [[ "$file_path" =~ (Anchor\.toml|\.env.*|constants\.rs)$ ]]; then
        log_info "Network configuration validation..."
        
        # Check for mixed network configurations
        if echo "$content" | grep -q "mainnet" && echo "$content" | grep -q "devnet\|localhost"; then
            log_warning "Mixed network configurations detected"
            log_info "Ensure consistent network settings across all configuration files"
        fi
        
        # Check for missing environment variable patterns
        if [[ "$file_path" =~ constants\.rs$ ]] && echo "$content" | grep -q "const.*PROGRAM_ID" && ! echo "$content" | grep -q "env!\|option_env!"; then
            log_warning "Hardcoded program ID detected"
            log_info "Consider using environment variables for network flexibility"
        fi
    fi
}

# Function to suggest MCP tool usage
suggest_mcp_tools() {
    local file_path="$1"
    local content="$2"
    
    # Suggest appropriate MCP tools based on content
    if echo "$content" | grep -q "account.*info\|getAccountInfo"; then
        log_info "💡 MCP Tool Suggestion: Use crypto-lottery getAccountInfo for enhanced account fetching"
    fi
    
    if echo "$content" | grep -q "instruction.*handler\|pub fn handler" && [[ "$file_path" =~ instructions/.*\.rs$ ]]; then
        log_info "💡 MCP Tool Suggestion: Use anchor-program-expert agent for complex instruction reviews"
    fi
    
    if echo "$content" | grep -q "#\[account\].*pub struct" && echo "$content" | grep -c "pub.*:" -gt 15; then
        log_info "💡 MCP Tool Suggestion: Use solana-stack-overflow-fixer for large struct optimization"
    fi
    
    if echo "$content" | grep -q "frontend\|typescript\|react" || [[ "$file_path" =~ \.(ts|tsx|js|jsx)$ ]]; then
        log_info "💡 MCP Tool Suggestion: Use generateTypesFromIDL for type synchronization"
    fi
}

# Main function
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local edit_content="${CLAUDE_EDIT_CONTENT:-}"
    
    # Only run on pre-edit hooks
    if [[ "$hook_type" != "pre-edit" ]]; then
        return 0
    fi
    
    # Skip for non-relevant files
    if [[ ! "$file_path" =~ \.(rs|toml|json|ts|tsx|js|jsx)$ ]]; then
        return 0
    fi
    
    log_info "Running MCP schema validation for: $(basename "$file_path")"
    
    # Run validations
    validate_idl_schema "$file_path" "$edit_content"
    validate_mcp_compatibility "$file_path" "$edit_content"
    validate_token_operations "$file_path" "$edit_content"
    validate_pda_operations "$file_path" "$edit_content"
    validate_network_config "$file_path" "$edit_content"
    suggest_mcp_tools "$file_path" "$edit_content"
    
    log_success "MCP schema validation completed successfully"
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi