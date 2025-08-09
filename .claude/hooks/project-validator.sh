#!/bin/bash

# Project Validator Hook for Solana Development
# Additional validations for project consistency

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [PROJECT-VALIDATOR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [PROJECT-VALIDATOR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [PROJECT-VALIDATOR]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [PROJECT-VALIDATOR]${NC} $1"
}

# Function to check for common Solana development issues
check_solana_environment() {
    log_info "Checking Solana environment..."
    
    # Check if solana CLI is available
    if ! command -v solana &> /dev/null; then
        log_warning "Solana CLI not found - install from https://docs.solana.com/cli/install-solana-cli-tools"
    else
        local solana_version=$(solana --version | cut -d' ' -f2)
        log_info "Solana CLI: $solana_version"
    fi
    
    # Check wallet configuration
    if [[ -f "$HOME/.config/solana/id.json" ]]; then
        log_success "Default wallet found"
    else
        log_warning "Default wallet not found - run 'solana-keygen new'"
    fi
}

# Function to check for development best practices
check_development_practices() {
    log_info "Checking development practices..."
    
    # Check for .gitignore
    if [[ -f ".gitignore" ]]; then
        # Check if common Solana/Anchor files are ignored
        if grep -q "target/" .gitignore && grep -q "node_modules/" .gitignore; then
            log_success ".gitignore configured properly"
        else
            log_warning ".gitignore might be missing Solana-specific entries"
            log_info "Consider adding: target/, node_modules/, test-ledger/"
        fi
    else
        log_warning ".gitignore not found"
    fi
    
    # Check for test-ledger cleanup
    if [[ -d "test-ledger" ]]; then
        log_warning "test-ledger directory found - consider adding to .gitignore"
    fi
    
    # Check for outdated files
    local backup_files=($(find . -name "*.bak" -o -name "*.backup" -o -name "*.old" 2>/dev/null))
    if [[ ${#backup_files[@]} -gt 0 ]]; then
        log_warning "Found ${#backup_files[@]} backup files:"
        for file in "${backup_files[@]}"; do
            log_info "  $file"
        done
        log_info "Consider cleaning up backup files"
    fi
}

# Function to validate network configuration
validate_network_config() {
    local target_network="${1:-localnet}"
    log_info "Validating network configuration for $target_network..."
    
    # Check Anchor.toml cluster setting
    if [[ -f "Anchor.toml" ]]; then
        local configured_cluster=$(grep -i 'cluster' Anchor.toml | cut -d'"' -f2 2>/dev/null | tr '[:upper:]' '[:lower:]')
        
        if [[ "$configured_cluster" == "$target_network" ]] || [[ "$configured_cluster" == "${target_network^}" ]]; then
            log_success "Anchor.toml cluster matches target: $target_network"
        else
            log_warning "Anchor.toml cluster ($configured_cluster) differs from target ($target_network)"
        fi
    fi
    
    # Check if localnet is running (for localnet deployments)
    if [[ "$target_network" == "localnet" ]]; then
        if pgrep -f "solana-test-validator" > /dev/null; then
            log_success "Local test validator is running"
        else
            log_warning "Local test validator not detected"
            log_info "Start with: solana-test-validator"
        fi
    fi
}

# Function to check program account sizes (prevent stack overflow)
check_program_sizes() {
    log_info "Checking for potential stack overflow issues..."
    
    # Look for large account structs in Rust files
    local large_structs=($(find programs -name "*.rs" -exec grep -l "pub struct.*{" {} \; 2>/dev/null))
    
    for file in "${large_structs[@]}"; do
        # Count fields in structs (rough heuristic)
        local field_count=$(grep -A 20 "pub struct" "$file" | grep -c "pub.*:" 2>/dev/null || echo 0)
        
        if [[ $field_count -gt 20 ]]; then
            log_warning "Large struct detected in $file (~$field_count fields)"
            log_info "Consider using Box<> for large account data to prevent stack overflow"
        fi
    done
}

# Main execution
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local command="${CLAUDE_COMMAND:-}"
    
    case "$hook_type" in
        "pre-bash")
            if [[ "$command" =~ deploy ]] || [[ "$command" =~ build ]]; then
                check_solana_environment
                validate_network_config
                check_program_sizes
            fi
            ;;
            
        "session-start")
            check_solana_environment
            check_development_practices
            validate_network_config "localnet"
            ;;
            
        *)
            # Default validation
            check_development_practices
            ;;
    esac
    
    log_success "Project validation completed"
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi