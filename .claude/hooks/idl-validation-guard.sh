#!/bin/bash

# IDL Validation Guard Hook
# Prevents IDL generation issues before they happen

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [IDL-GUARD]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [IDL-GUARD]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [IDL-GUARD]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [IDL-GUARD]${NC} $1"
}

# Function to validate instruction module exports
validate_instruction_exports() {
    local file_path="$1"
    local content="$2"
    
    if [[ "$file_path" =~ instructions/mod\.rs$ ]]; then
        log_info "Validating instruction module exports for IDL generation..."
        
        # Check for handler function exports
        local handler_count=$(echo "$content" | grep -c "handler" || echo 0)
        local pub_use_count=$(echo "$content" | grep -c "pub use" || echo 0)
        
        if [[ $pub_use_count -gt 5 && $handler_count -eq 0 ]]; then
            log_error "BLOCKED: Instruction modules detected but no handler exports found"
            log_error "This will cause IDL generation to fail with 'handler not found' errors"
            echo ""
            log_info "✅ Solution: Ensure each instruction module exports its handler:"
            log_info "   pub use module_name::{StructName, handler as module_handler};"
            echo ""
            exit 1
        fi
        
        # Check for missing module references in lib.rs
        if echo "$content" | grep -q "pub use.*::.*;" && [[ "$file_path" =~ roulette ]]; then
            log_warning "Large instruction module - ensure all exports are referenced in lib.rs"
        fi
        
        log_success "Instruction module exports validation passed"
    fi
}

# Function to validate instruction file structure
validate_instruction_files() {
    local file_path="$1" 
    local content="$2"
    
    if [[ "$file_path" =~ instructions/[^/]*\.rs$ ]] && [[ ! "$file_path" =~ mod\.rs$ ]]; then
        log_info "Validating instruction file: $(basename "$file_path")"
        
        # Check for required handler function
        if ! echo "$content" | grep -q "pub fn handler\|pub fn.*handler"; then
            log_error "BLOCKED: Instruction file missing handler function"
            log_error "File: $(basename "$file_path")"
            log_error "IDL generation requires each instruction to have a handler function"
            echo ""
            log_info "✅ Solution: Add a public handler function:"
            log_info "   pub fn handler(ctx: Context<YourStruct>) -> Result<()> { ... }"
            echo ""
            exit 1
        fi
        
        # Check for required imports
        if ! echo "$content" | grep -q "use anchor_lang::prelude::*"; then
            log_warning "Missing anchor_lang prelude import in $(basename "$file_path")"
        fi
        
        # Check for error imports if using Result types
        if echo "$content" | grep -q "Result<.*>" && ! echo "$content" | grep -q "use crate::errors"; then
            log_warning "Using Result types but missing error imports in $(basename "$file_path")"
        fi
        
        log_success "Instruction file validation passed: $(basename "$file_path")"
    fi
}

# Function to validate lib.rs program structure  
validate_program_structure() {
    local file_path="$1"
    local content="$2"
    
    if [[ "$file_path" =~ lib\.rs$ ]] && echo "$content" | grep -q "#\[program\]"; then
        log_info "Validating program structure for IDL generation..."
        
        # Check for instruction function references
        local instruction_count=$(echo "$content" | grep -c "pub fn.*ctx: Context<.*>" || echo 0)
        
        if [[ $instruction_count -eq 0 ]]; then
            log_error "BLOCKED: No instruction functions found in lib.rs"
            log_error "IDL generation requires publicly exposed instruction functions"
            echo ""
            log_info "✅ Solution: Ensure instruction handlers are public in lib.rs:"
            log_info "   pub fn instruction_name(ctx: Context<StructName>) -> Result<()>"
            echo ""
            exit 1
        fi
        
        # Check for unused instruction imports
        if echo "$content" | grep -q "use.*instructions::" && echo "$content" | grep -q "//.*"; then
            log_warning "Commented out instruction imports detected - may cause IDL issues"
            log_info "Clean up unused imports or use #[cfg(feature = \"..\")] guards"
        fi
        
        # Check for missing error module
        if ! echo "$content" | grep -q "pub use.*errors"; then
            log_warning "Error module not exposed - may cause IDL type issues"
        fi
        
        log_success "Program structure validation passed"
    fi
}

# Function to check for common IDL-breaking patterns
validate_idl_compatibility() {
    local file_path="$1"
    local content="$2"
    
    if [[ "$file_path" =~ \.rs$ ]]; then
        # Check for conflicting struct names
        if echo "$content" | grep -q "pub struct.*Account" && echo "$content" | grep -q "pub enum.*Account"; then
            log_error "BLOCKED: Conflicting Account struct/enum names detected"
            log_error "This causes IDL generation conflicts"
            echo ""
            log_info "✅ Solution: Use distinct names like YourStruct vs YourEnum"
            echo ""
            exit 1
        fi
        
        # Check for missing derives on public structs
        if echo "$content" | grep -q "pub struct" && ! echo "$content" | grep -q "#\[derive.*\]"; then
            log_warning "Public struct without derives may not appear in IDL"
        fi
        
        # Check for invalid instruction parameters
        if echo "$content" | grep -q "pub fn handler.*&.*," && echo "$content" | grep -q "Context<.*>"; then
            log_error "BLOCKED: Handler function with invalid parameter types"
            log_error "References (&) in handler parameters break IDL generation"
            echo ""
            log_info "✅ Solution: Use owned types in handler parameters"
            echo ""
            exit 1
        fi
    fi
}

# Function to validate cargo dependencies
validate_cargo_dependencies() {
    local file_path="$1"
    local content="$2"
    
    if [[ "$file_path" =~ Cargo\.toml$ ]] && echo "$content" | grep -q "\[dependencies\]"; then
        log_info "Validating Cargo dependencies for IDL compatibility..."
        
        # Check for anchor version consistency
        if echo "$content" | grep -q "anchor-lang.*=" && echo "$content" | grep -q "anchor-spl.*="; then
            local anchor_lang_version=$(echo "$content" | grep "anchor-lang" | grep -o '"[0-9.]*"' | head -1)
            local anchor_spl_version=$(echo "$content" | grep "anchor-spl" | grep -o '"[0-9.]*"' | head -1)
            
            if [[ "$anchor_lang_version" != "$anchor_spl_version" ]] && [[ -n "$anchor_lang_version" ]] && [[ -n "$anchor_spl_version" ]]; then
                log_error "BLOCKED: Mismatched Anchor versions detected"
                log_error "anchor-lang: $anchor_lang_version vs anchor-spl: $anchor_spl_version"
                log_error "Version mismatches cause IDL generation failures"
                echo ""
                log_info "✅ Solution: Use consistent Anchor versions across all dependencies"
                echo ""
                exit 1
            fi
        fi
        
        # Check for conflicting solana-program versions
        if echo "$content" | grep -q "solana-program.*=" && echo "$content" | grep -c "solana-program" -gt 1; then
            log_warning "Multiple solana-program dependencies may cause version conflicts"
        fi
        
        log_success "Cargo dependencies validation passed"
    fi
}

# Function to suggest IDL health improvements
suggest_idl_improvements() {
    local file_path="$1"
    local content="$2"
    
    if [[ "$file_path" =~ instructions/.*\.rs$ ]]; then
        log_info "💡 IDL Health Tips for: $(basename "$file_path")"
        
        # Suggest handler naming consistency
        if echo "$content" | grep -q "pub fn handler"; then
            log_info "   ✓ Consider using descriptive handler names for better IDL docs"
        fi
        
        # Suggest proper documentation
        if ! echo "$content" | grep -q "///.*"; then
            log_info "   📚 Add doc comments for better IDL documentation"
        fi
        
        # Suggest error handling
        if echo "$content" | grep -q "unwrap()" || echo "$content" | grep -q "expect("; then
            log_info "   🛡️  Consider using proper error handling instead of unwrap/expect"
        fi
    fi
}

# Main validation function
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local edit_content="${CLAUDE_EDIT_CONTENT:-}"
    
    # Only run on pre-edit hooks for Rust files
    if [[ "$hook_type" != "pre-edit" ]] || [[ ! "$file_path" =~ \.(rs|toml)$ ]]; then
        return 0
    fi
    
    log_info "Running IDL validation guard for: $(basename "$file_path")"
    
    # Run all validations
    validate_instruction_exports "$file_path" "$edit_content"
    validate_instruction_files "$file_path" "$edit_content" 
    validate_program_structure "$file_path" "$edit_content"
    validate_idl_compatibility "$file_path" "$edit_content"
    validate_cargo_dependencies "$file_path" "$edit_content"
    suggest_idl_improvements "$file_path" "$edit_content"
    
    log_success "IDL validation guard completed successfully"
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi