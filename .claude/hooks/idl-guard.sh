#!/bin/bash

# IDL Guard Hook - Consolidated IDL Generation Protection
# Replaces: idl-compilation-guard.sh, idl-validation-guard.sh, anchor-only-idl-guard.sh
# Focuses on preventing the critical pattern: Option<Account<'info, Mint/TokenAccount>>
# Triggers: pre-edit (Rust files only), pre-bash (build commands), session-start

set -e

# Cache configuration
CACHE_DIR=".claude/cache"
CACHE_EXPIRY=180  # 3 minutes (shorter for code changes)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
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

# Cache management
init_cache() {
    mkdir -p "$CACHE_DIR"
}

get_file_hash() {
    if [[ -f "$1" ]]; then
        # Use file modification time and size as a simple hash
        stat -c "%Y%s" "$1" 2>/dev/null || stat -f "%m%z" "$1" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

is_file_cache_valid() {
    local file_path="$1"
    local cache_file="$CACHE_DIR/idl_$(basename "$file_path" | tr '/' '_')_cache"
    
    if [[ -f "$cache_file" ]]; then
        local cached_hash=$(cat "$cache_file" 2>/dev/null || echo "")
        local current_hash=$(get_file_hash "$file_path")
        [[ "$cached_hash" == "$current_hash" ]]
    else
        false
    fi
}

cache_file_result() {
    local file_path="$1"
    local cache_file="$CACHE_DIR/idl_$(basename "$file_path" | tr '/' '_')_cache"
    local file_hash=$(get_file_hash "$file_path")
    echo "$file_hash" > "$cache_file"
}

# CRITICAL: Detect the exact patterns that caused original IDL failures
check_critical_idl_patterns() {
    local file="$1"
    local critical_issues_found=0
    
    if [[ ! -f "$file" ]]; then
        return 0
    fi
    
    # Check cache first
    if is_file_cache_valid "$file"; then
        return 0
    fi
    
    log_info "Scanning $file for critical IDL patterns..."
    
    # CRITICAL PATTERN 1: Option<Account<'info, Mint>>
    if grep -q "Option<Account<'info,\s*Mint>>" "$file" 2>/dev/null; then
        log_error "🔴 CRITICAL: Optional Account<'info, Mint> detected in $(basename "$file")"
        log_error "   This WILL cause IDL generation failure: missing trait implementations"
        log_error "   Fix: Use 'mint: Pubkey' instead of 'Option<Account<'info, Mint>>'"
        critical_issues_found=$((critical_issues_found + 1))
    fi
    
    # CRITICAL PATTERN 2: Option<Account<'info, TokenAccount>>
    if grep -q "Option<Account<'info,\s*TokenAccount>>" "$file" 2>/dev/null; then
        log_error "🔴 CRITICAL: Optional Account<'info, TokenAccount> detected in $(basename "$file")"
        log_error "   This WILL cause IDL generation failure: missing trait implementations"
        log_error "   Fix: Use 'token_account: Pubkey' instead of 'Option<Account<'info, TokenAccount>>'"
        critical_issues_found=$((critical_issues_found + 1))
    fi
    
    # CRITICAL PATTERN 3: Direct anchor_spl imports in structs
    if grep -A 5 -B 5 "#\[derive(Accounts)\]" "$file" 2>/dev/null | grep -q "use anchor_spl::token::{Mint\|TokenAccount}"; then
        log_warning "⚠️  WARNING: Direct anchor_spl imports near Account struct in $(basename "$file")"
        log_warning "   This can cause IDL issues if types don't implement required traits"
        log_warning "   Consider using Pubkey for optional accounts"
    fi
    
    # CRITICAL PATTERN 4: Large struct detection (stack overflow prevention)
    local struct_field_count=$(grep -A 50 "#\[derive(Accounts)\]" "$file" 2>/dev/null | grep -c "pub.*:" 2>/dev/null || echo 0)
    struct_field_count=${struct_field_count:-0}
    if [[ $struct_field_count -gt 25 ]]; then
        log_warning "⚠️  WARNING: Large account struct detected in $(basename "$file") ($struct_field_count fields)"
        log_warning "   This could cause stack overflow. Consider using Box<> for large structs"
    fi
    
    if [[ $critical_issues_found -eq 0 ]]; then
        cache_file_result "$file"
    fi
    
    return $critical_issues_found
}

# Quick Anchor configuration validation
validate_anchor_config() {
    local cache_file="$CACHE_DIR/anchor_config_cache"
    
    if [[ -f "$cache_file" ]] && [[ $(find "$cache_file" -mmin -3 2>/dev/null) ]]; then
        local cached_result=$(cat "$cache_file" 2>/dev/null || echo "")
        if [[ "$cached_result" == "VALID" ]]; then
            return 0
        fi
    fi
    
    log_info "Quick Anchor configuration check..."
    
    if [[ ! -f "Anchor.toml" ]]; then
        return 0  # Not an Anchor project
    fi
    
    # Check for known problematic proc_macro2 versions
    if find . -name "Cargo.toml" -exec grep -l 'proc-macro2.*1\.0\.9[5-9]' {} \; 2>/dev/null | head -1 | read -r problematic_cargo; then
        log_error "🔴 CRITICAL: Problematic proc_macro2 version detected in $problematic_cargo"
        log_error "   Versions 1.0.95+ cause IDL generation failures with 'source_file' errors"
        log_error "   Fix: Add 'proc-macro2 = \"=1.0.94\"' to Cargo.toml"
        return 1
    fi
    
    # Check for missing idl-build features in programs using anchor-spl
    local programs_with_anchor_spl=$(find programs -name "Cargo.toml" -exec grep -l "anchor-spl" {} \; 2>/dev/null || true)
    for program_cargo in $programs_with_anchor_spl; do
        if ! grep -A 5 "\[features\]" "$program_cargo" | grep -q "idl-build.*anchor-spl/idl-build" 2>/dev/null; then
            log_warning "⚠️  WARNING: Missing idl-build feature in $program_cargo"
            log_warning "   Add 'idl-build = [\"anchor-spl/idl-build\"]' to prevent cryptic errors"
        fi
    done
    
    # ENHANCED IDL GENERATION CHECKS
    check_idl_generation_health
    check_frontend_idl_sync
    
    echo "VALID" > "$cache_file"
    log_success "Anchor configuration validated"
    return 0
}

# Enhanced IDL Generation Health Checks
check_idl_generation_health() {
    log_info "Checking IDL generation health..."
    
    # Check if target/idl directory exists but is empty
    if [[ -d "target/idl" ]]; then
        local idl_count=$(find target/idl -name "*.json" 2>/dev/null | wc -l | tr -d ' ')
        if [[ $idl_count -eq 0 ]]; then
            log_warning "⚠️  IDL directory exists but is empty"
            log_warning "   This may indicate IDL generation issues"
            log_warning "   Try: npm run build:reliable"
        fi
    fi
    
    # Check Rust toolchain compatibility
    if command -v rustc >/dev/null 2>&1; then
        local rust_version=$(rustc --version | grep -oE "1\.[0-9]+")
        if [[ "$rust_version" < "1.78" ]]; then
            log_warning "⚠️  Rust version $rust_version may cause IDL issues"
            log_warning "   Consider updating to stable toolchain"
        fi
    fi
    
    # Check for Anchor version compatibility
    if command -v anchor >/dev/null 2>&1; then
        local anchor_version=$(anchor --version | grep -oE "[0-9]+\.[0-9]+\.[0-9]+")
        if [[ -f "Anchor.toml" ]]; then
            local config_version=$(grep 'anchor_version' Anchor.toml | cut -d'"' -f2 2>/dev/null || echo "")
            if [[ -n "$config_version" && "$anchor_version" != "$config_version" ]]; then
                log_warning "⚠️  Anchor CLI ($anchor_version) != Config ($config_version)"
                log_warning "   Version mismatches can cause IDL generation failures"
            fi
        fi
    fi
}

# Check Frontend IDL Synchronization
check_frontend_idl_sync() {
    local frontend_path="../crypto-lottery-frontend/src/lib/solana"
    
    if [[ ! -d "$frontend_path" ]]; then
        return 0  # Frontend not present, skip check
    fi
    
    log_info "Checking frontend IDL synchronization..."
    
    # Check if IDL files exist in frontend
    local program_names=("decentralized_lottery" "decentralized_roulette")
    for program in "${program_names[@]}"; do
        local target_idl="target/idl/${program}.json"
        local frontend_idl="$frontend_path/${program}.json"
        
        if [[ -f "$target_idl" && -f "$frontend_idl" ]]; then
            # Check file sizes (rough sync check)
            local target_size=$(stat -f%z "$target_idl" 2>/dev/null || echo "0")
            local frontend_size=$(stat -f%z "$frontend_idl" 2>/dev/null || echo "0")
            
            local size_diff=$((target_size - frontend_size))
            if [[ $size_diff -lt -1000 || $size_diff -gt 1000 ]]; then
                log_warning "⚠️  IDL size mismatch for $program"
                log_warning "   Target: ${target_size}B, Frontend: ${frontend_size}B"
                log_warning "   Consider running: npm run sync-idl"
            fi
            
            # Check if addresses match
            if command -v jq >/dev/null 2>&1; then
                local target_addr=$(jq -r '.address // .metadata.address // empty' "$target_idl" 2>/dev/null || echo "")
                local frontend_addr=$(jq -r '.address // .metadata.address // empty' "$frontend_idl" 2>/dev/null || echo "")
                
                if [[ -n "$target_addr" && -n "$frontend_addr" && "$target_addr" != "$frontend_addr" ]]; then
                    log_error "🔴 CRITICAL: Address mismatch in $program IDL"
                    log_error "   Target: $target_addr"
                    log_error "   Frontend: $frontend_addr"
                    return 1
                fi
            fi
        elif [[ -f "$target_idl" && ! -f "$frontend_idl" ]]; then
            log_warning "⚠️  IDL exists in target but missing in frontend: $program"
            log_warning "   Run: npm run sync-idl"
        fi
    done
    
    return 0
}

# Block dangerous manual IDL operations
block_manual_idl_operations() {
    local command="${CLAUDE_COMMAND:-${CLAUDE_BASH_COMMAND:-}}"
    
    # Block dangerous manual IDL commands
    if [[ "$command" =~ anchor[[:space:]]+idl[[:space:]]+(build|extract) ]] && [[ ! "$command" =~ anchor[[:space:]]+build ]]; then
        log_error "🔴 BLOCKED: Manual IDL generation command detected"
        log_error "   Command: $command"
        log_error "   Manual IDL operations are blocked to prevent failures"
        log_error "   Use 'anchor build' instead - it handles IDL generation safely"
        return 1
    fi
    
    if [[ "$command" =~ npm[[:space:]]+run[[:space:]]+(generate-idl|update-idl|extract-idl) ]]; then
        log_error "🔴 BLOCKED: Manual IDL script detected"
        log_error "   Command: $command"
        log_error "   Manual IDL scripts are blocked to prevent failures"
        log_error "   Use 'anchor build' or 'npm run build' instead"
        return 1
    fi
    
    return 0
}

# Determine what IDL validation is needed
determine_idl_validation_scope() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local command="${CLAUDE_COMMAND:-${CLAUDE_BASH_COMMAND:-}}"
    
    case "$hook_type" in
        "pre-edit")
            # Only scan Rust files in programs directory
            if [[ "$file_path" =~ \.rs$ ]] && [[ "$file_path" =~ programs/ ]]; then
                echo "critical_patterns"
            elif [[ "$file_path" =~ (Cargo\.toml|Anchor\.toml)$ ]]; then
                echo "config"
            else
                echo "none"
            fi
            ;;
        "pre-bash")
            if [[ "$command" =~ (anchor.*build|anchor.*deploy|npm.*(build|deploy)) ]]; then
                echo "config block_manual"
            elif [[ "$command" =~ anchor.*idl ]]; then
                echo "block_manual"
            else
                echo "none"
            fi
            ;;
        "session-start"|"project-start")
            echo "config"
            ;;
        *)
            echo "none"
            ;;
    esac
}

# Main execution logic
main() {
    init_cache
    
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local validation_scope=$(determine_idl_validation_scope)
    
    log_info "IDL protection triggered: $hook_type"
    
    if [[ "$validation_scope" == "none" ]]; then
        exit 0
    fi
    
    local validations_needed=($validation_scope)
    
    for validation in "${validations_needed[@]}"; do
        case "$validation" in
            "critical_patterns")
                if [[ -n "$file_path" ]]; then
                    if ! check_critical_idl_patterns "$file_path"; then
                        log_error "Critical IDL patterns detected in $file_path"
                        log_error "Fix these issues before proceeding to prevent IDL generation failures"
                        exit 1
                    fi
                fi
                ;;
            "config")
                if ! validate_anchor_config; then
                    log_error "Anchor configuration issues detected"
                    exit 1
                fi
                ;;
            "block_manual")
                if ! block_manual_idl_operations; then
                    log_error "Manual IDL operation blocked"
                    exit 1
                fi
                ;;
        esac
    done
    
    log_success "IDL protection validated"
}

# Run main function if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi