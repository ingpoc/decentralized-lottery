#!/bin/bash

# Enhanced Solana Development Validator Hook
# Prevents common Solana/Anchor development issues before they become compilation errors

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [SOLANA-VALIDATOR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [SOLANA-VALIDATOR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [SOLANA-VALIDATOR]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [SOLANA-VALIDATOR]${NC} $1"
}

log_blocked() {
    echo -e "${PURPLE}🛡️  [SOLANA-VALIDATOR]${NC} $1"
}

# Function to validate instruction module exports
validate_instruction_exports() {
    local file_path="$1"
    local content="$2"
    
    # Check if this is an instruction mod.rs file
    if [[ "$file_path" =~ instructions/mod\.rs$ ]]; then
        log_info "Validating instruction module exports..."
        
        # Count wildcard exports
        local wildcard_exports=$(echo "$content" | grep -c "pub use.*::.*;" || echo 0)
        
        # Check for ambiguous handler exports
        local handler_exports=$(echo "$content" | grep -c "pub use.*::handler" || echo 0)
        
        if [[ $wildcard_exports -gt 10 && $handler_exports -eq 0 ]]; then
            log_error "BLOCKED: Too many wildcard exports ($wildcard_exports) detected in instruction module"
            log_error "This will cause ambiguous 'handler' function conflicts during compilation"
            echo ""
            log_info "✅ Solution: Use specific exports with aliases:"
            log_info "   Before: pub use initialize::*;"
            log_info "   After:  pub use initialize::{Initialize, handler as initialize_handler};"
            echo ""
            log_info "This prevents the exact error you encountered: 'ambiguous glob re-exports'"
            exit 1
        fi
        
        # Check for proper handler aliasing if wildcard exports exist
        if [[ $wildcard_exports -gt 5 ]]; then
            if ! echo "$content" | grep -q "handler as.*_handler"; then
                log_warning "Consider using handler aliases to prevent naming conflicts"
                log_info "Example: pub use module::{Struct, handler as module_handler};"
            fi
        fi
        
        log_success "Instruction module exports validation passed"
    fi
}

# Function to validate account struct field access
validate_account_field_access() {
    local file_path="$1"
    local content="$2"
    
    # Check for direct field access patterns that should use helper methods
    if echo "$content" | grep -q "\.completed_at\s*=\s*Some"; then
        log_error "BLOCKED: Direct assignment to 'completed_at' field detected"
        log_error "This field should be set using the helper method"
        echo ""
        log_info "✅ Solution: Use helper method instead:"
        log_info "   Before: lottery_account.completed_at = Some(timestamp);"
        log_info "   After:  lottery_account.mark_completed(timestamp);"
        echo ""
        exit 1
    fi
    
    # Check for direct flag field access
    if echo "$content" | grep -q "\.is_prize_pool_locked\s*="; then
        log_error "BLOCKED: Direct assignment to 'is_prize_pool_locked' field detected"
        log_error "This field is managed through flags and should use helper methods"
        echo ""
        log_info "✅ Solution: Use helper method instead:"
        log_info "   Before: account.is_prize_pool_locked = true;"
        log_info "   After:  account.set_is_prize_pool_locked(true);"
        echo ""
        exit 1
    fi
    
    # Check for auto_transition field access
    if echo "$content" | grep -q "\.auto_transition\s*="; then
        log_error "BLOCKED: Direct assignment to 'auto_transition' field detected" 
        log_error "This field is managed through flags and should use helper methods"
        echo ""
        log_info "✅ Solution: Use helper method instead:"
        log_info "   Before: account.auto_transition = true;"
        log_info "   After:  account.set_auto_transition(true);"
        echo ""
        exit 1
    fi
}

# Function to validate account struct size for stack overflow prevention
validate_account_struct_size() {
    local file_path="$1"
    local content="$2"
    
    # Check if this is an account struct definition
    if [[ "$file_path" =~ state/.*\.rs$ ]] && echo "$content" | grep -q "#\[account\]"; then
        log_info "Validating account struct for stack overflow prevention..."
        
        # Count fields in account structs
        local field_count=$(echo "$content" | grep -c "pub [a-z_]*:" || echo 0)
        
        if [[ $field_count -gt 20 ]]; then
            log_error "BLOCKED: Account struct has $field_count fields (>20)"
            log_error "Large structs can cause stack overflow errors in Solana programs"
            log_error "Stack limit is 4096 bytes - your struct may exceed this"
            echo ""
            log_info "✅ Solutions:"
            log_info "1. Use Box<> for large nested data: pub large_data: Box<LargeStruct>,"
            log_info "2. Split into smaller structs with references"
            log_info "3. Use account compression techniques"
            log_info "4. Move large data to separate accounts"
            echo ""
            log_warning "Run 'solana-stack-overflow-fixer' agent for automated fixes"
            exit 1
        fi
        
        # Check for nested large structs without Box<>
        if echo "$content" | grep -q "pub.*: \[.*; [0-9]\{3,\}\]"; then
            log_warning "Large array detected - consider using Box<> or separate account"
        fi
        
        log_success "Account struct size validation passed ($field_count fields)"
    fi
}

# Function to validate proper PDA derivation patterns
validate_pda_patterns() {
    local file_path="$1"
    local content="$2"
    
    # Check for hardcoded seeds that should use constants
    if echo "$content" | grep -q 'seeds = \[b"[^"]*"' && ! echo "$content" | grep -q "SEED\|_SEED"; then
        log_warning "Consider using seed constants instead of hardcoded strings"
        log_info "Example: const LOTTERY_SEED: &[u8] = b\"lottery\";"
    fi
    
    # Check for missing bump parameter in PDA derivations
    if echo "$content" | grep -q "seeds = \[" && ! echo "$content" | grep -q "bump"; then
        log_warning "PDA derivation without bump parameter detected"
        log_info "Add 'bump' parameter to account constraint for security"
    fi
}

# Function to validate proper error handling patterns
validate_error_handling() {
    local file_path="$1" 
    local content="$2"
    
    # Check for unwrap() usage in program code
    if [[ "$file_path" =~ programs/.*/src/ ]] && echo "$content" | grep -q "\.unwrap()"; then
        log_error "BLOCKED: .unwrap() usage detected in program code"
        log_error "Unwrap can cause program panics - use proper error handling"
        echo ""
        log_info "✅ Solutions:"
        log_info "1. Use .map_err(|_| ProgramError::CustomError)?"
        log_info "2. Use match statements with proper error returns"
        log_info "3. Use .ok_or(ProgramError::CustomError)?"
        echo ""
        exit 1
    fi
    
    # Check for proper Result types in handler functions
    if [[ "$file_path" =~ instructions/.*\.rs$ ]] && echo "$content" | grep -q "pub fn handler" && ! echo "$content" | grep -q "-> Result<"; then
        log_warning "Handler function should return Result<()> for proper error handling"
    fi
}

# Function to validate proper anchor version compatibility
validate_anchor_patterns() {
    local file_path="$1"
    local content="$2"
    
    # Check for outdated bump syntax
    if echo "$content" | grep -q "\.get(\".*\")"; then
        log_error "BLOCKED: Outdated bumps API syntax detected"
        log_error "Using old .get() method which doesn't exist in Anchor 0.30.1"
        echo ""
        log_info "✅ Solution: Use direct field access instead:"
        log_info "   Before: ctx.bumps.get(\"account_name\")"
        log_info "   After:  ctx.bumps.account_name"
        echo ""
        exit 1
    fi
    
    # Check for deprecated Account import patterns
    if echo "$content" | grep -q "use anchor_lang::Account"; then
        log_warning "Consider using anchor_lang::prelude::* for cleaner imports"
    fi
}

# Function to validate instruction complexity
validate_instruction_complexity() {
    local file_path="$1"
    local content="$2"
    
    if [[ "$file_path" =~ instructions/.*\.rs$ ]]; then
        # Count lines in handler function to estimate complexity
        local handler_lines=$(echo "$content" | sed -n '/pub fn handler/,/^}/p' | wc -l || echo 0)
        
        if [[ $handler_lines -gt 200 ]]; then
            log_warning "Large instruction handler ($handler_lines lines) - consider refactoring"
            log_info "Large handlers can cause stack overflow - split into smaller functions"
        fi
        
        # Check for too many accounts in single instruction
        local account_count=$(echo "$content" | grep -c "#\[account(" || echo 0)
        
        if [[ ${account_count:-0} -gt 15 ]]; then
            log_warning "Many accounts ($account_count) in single instruction - may hit account limits"
        fi
    fi
}

# Main validation function
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local edit_content="${CLAUDE_EDIT_CONTENT:-}"
    
    # Only run on pre-edit hooks for Solana program files
    if [[ "$hook_type" != "pre-edit" ]]; then
        return 0
    fi
    
    # Only validate Solana program files
    if [[ ! "$file_path" =~ programs/.*/src/ ]]; then
        return 0
    fi
    
    log_info "Validating Solana development patterns for: $(basename "$file_path")"
    
    # Run all validations
    validate_instruction_exports "$file_path" "$edit_content"
    validate_account_field_access "$file_path" "$edit_content"
    validate_account_struct_size "$file_path" "$edit_content"
    validate_pda_patterns "$file_path" "$edit_content"
    validate_error_handling "$file_path" "$edit_content"
    validate_anchor_patterns "$file_path" "$edit_content"
    validate_instruction_complexity "$file_path" "$edit_content"
    
    log_success "All Solana development pattern validations passed"
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi